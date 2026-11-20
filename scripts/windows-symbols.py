"""Validate and retain matching Windows PE/PDB files without uploading to Sentry."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import uuid


def read_at(data, offset, size):
    if offset < 0 or size < 0 or offset + size > len(data):
        raise ValueError("truncated debug information")
    return data[offset:offset + size]


def u32(data, offset):
    return struct.unpack("<I", read_at(data, offset, 4))[0]


def pe_identity(data):
    if read_at(data, 0, 2) != b"MZ":
        raise ValueError("expected a PE executable")
    pe = u32(data, 0x3c)
    if read_at(data, pe, 4) != b"PE\0\0":
        raise ValueError("invalid PE signature")
    count = struct.unpack("<H", read_at(data, pe + 6, 2))[0]
    optional_size = struct.unpack("<H", read_at(data, pe + 20, 2))[0]
    optional = pe + 24
    magic = struct.unpack("<H", read_at(data, optional, 2))[0]
    directories = {0x10b: 96, 0x20b: 112}.get(magic)
    if directories is None or optional_size < directories + 7 * 8:
        raise ValueError("missing PE debug directory")
    debug_rva = u32(data, optional + directories + 6 * 8)
    debug_size = u32(data, optional + directories + 6 * 8 + 4)
    sections = optional + optional_size
    for index in range(count):
        section = sections + index * 40
        va = u32(data, section + 12)
        raw_size = u32(data, section + 16)
        if va <= debug_rva and debug_rva + debug_size <= va + raw_size:
            start = u32(data, section + 20) + debug_rva - va
            break
    else:
        raise ValueError("debug directory is not in a file section")
    if debug_size % 28:
        raise ValueError("invalid PE debug directory size")
    for offset in range(start, start + debug_size, 28):
        if u32(data, offset + 12) == 2:  # IMAGE_DEBUG_TYPE_CODEVIEW
            record = read_at(data, u32(data, offset + 24), u32(data, offset + 16))
            if record[:4] == b"RSDS":
                return str(uuid.UUID(bytes_le=read_at(record, 4, 16))), u32(record, 20)
    raise ValueError("PE has no RSDS identity")


def pdb_identity(data):
    if read_at(data, 0, 32) != b"Microsoft C/C++ MSF 7.00\r\n\x1aDS\0\0\0":
        raise ValueError("expected an MSF 7 PDB")
    block_size = u32(data, 32)
    directory_size = u32(data, 44)
    if block_size not in (512, 1024, 2048, 4096):
        raise ValueError("unsupported PDB block size")
    directory_count = (directory_size + block_size - 1) // block_size
    if directory_count * 4 > block_size:
        raise ValueError("unsupported PDB directory block map")
    block_map = u32(data, 52) * block_size

    def blocks(numbers, size):
        return b"".join(read_at(data, number * block_size, block_size)
                        for number in numbers)[:size]

    directory = blocks([u32(data, block_map + i * 4)
                        for i in range(directory_count)], directory_size)
    streams = u32(directory, 0)
    if streams < 2 or streams > len(directory) // 4 - 1:
        raise ValueError("missing PDB info stream")
    sizes = [u32(directory, 4 + i * 4) for i in range(streams)]
    cursor = 4 + streams * 4
    for index, size in enumerate(sizes[:2]):
        count = 0 if size == 0xffffffff else (size + block_size - 1) // block_size
        pages = [u32(directory, cursor + i * 4) for i in range(count)]
        cursor += count * 4
        if index == 1:
            info = blocks(pages, size)
            return str(uuid.UUID(bytes_le=read_at(info, 12, 16))), u32(info, 8)
    raise ValueError("missing PDB identity")


def collect(binary, pdb, output, revision):
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("revision must be a full lowercase Git SHA")
    exe_data, pdb_data = binary.read_bytes(), pdb.read_bytes()
    identity = pe_identity(exe_data)
    if identity != pdb_identity(pdb_data):
        raise ValueError("PE/PDB GUID or age mismatch; refusing to retain unrelated symbols")
    # Dedicated directory, never the public release-assets directory. Refuse to
    # overwrite earlier evidence or partially populated output.
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError("symbol output directory must be empty")
    if binary.name == pdb.name or "manifest.json" in (binary.name, pdb.name):
        raise ValueError("symbol filenames must be distinct")
    manifest = {"schema": 1, "revision": revision,
                "debug_id": f"{identity[0]}-{identity[1]:x}", "files": {}}
    for source, data in [(binary, exe_data), (pdb, pdb_data)]:
        manifest["files"][source.name] = {"sha256": hashlib.sha256(data).hexdigest(),
                                         "size": len(data)}
        # Copy exactly the bytes validated above, even if a build later replaces
        # the input path. No source paths or PDB contents enter the manifest.
        (output / source.name).write_bytes(data)
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--pdb", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()
    try:
        manifest = collect(args.binary, args.pdb, args.output, args.revision)
        print(json.dumps(manifest))
    except (OSError, ValueError, struct.error) as error:
        parser.exit(1, f"Windows symbols: {error}\n")
