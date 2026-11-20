"""PE debug-directory and PDB info-stream matching, including rejection paths."""
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import uuid

spec = importlib.util.spec_from_file_location("windows_symbols", Path(__file__).with_name("windows-symbols.py"))
symbols = importlib.util.module_from_spec(spec)
spec.loader.exec_module(symbols)
GUID = uuid.UUID("ad314f0a-07ac-458d-841f-cd1d45e16ba1")


def executable(age=1):
    data = bytearray(1024)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3c, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<H", data, 0x86, 1)
    struct.pack_into("<H", data, 0x94, 240)
    struct.pack_into("<H", data, 0x98, 0x20b)
    struct.pack_into("<II", data, 0x98 + 112 + 48, 0x1000, 28)
    struct.pack_into("<IIII", data, 0x98 + 240 + 8, 512, 0x1000, 512, 512)
    struct.pack_into("<I", data, 512 + 12, 2)
    record = b"RSDS" + GUID.bytes_le + struct.pack("<I", age) + b"private/build/jajan.pdb\0"
    struct.pack_into("<I", data, 512 + 16, len(record))
    struct.pack_into("<I", data, 512 + 24, 600)
    data[600:600 + len(record)] = record
    # A byte search is insufficient: executable instructions can contain RSDS.
    data[64:88] = b"RSDS" + bytes(20)
    return bytes(data)


def pdb(age=1):
    data = bytearray(512 * 5)
    data[:32] = b"Microsoft C/C++ MSF 7.00\r\n\x1aDS\0\0\0"
    struct.pack_into("<IIIIII", data, 32, 512, 1, 5, 16, 0, 2)
    struct.pack_into("<I", data, 512 * 2, 3)
    struct.pack_into("<IIII", data, 512 * 3, 2, 0, 28, 4)
    struct.pack_into("<III", data, 512 * 4, 20000404, 0, age)
    data[512 * 4 + 12:512 * 4 + 28] = GUID.bytes_le
    return bytes(data)


class SymbolsTest(unittest.TestCase):
    def test_reads_directory_instead_of_first_rsds_bytes(self):
        self.assertEqual(symbols.pe_identity(executable()), (str(GUID), 1))
        self.assertEqual(symbols.pdb_identity(pdb()), (str(GUID), 1))

    def test_retains_exact_bytes_and_no_input_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exe, debug, out = root / "jajan.exe", root / "jajan.pdb", root / "retained"
            exe.write_bytes(executable())
            debug.write_bytes(pdb())
            manifest = symbols.collect(exe, debug, out, "a" * 40)
            self.assertEqual((out / exe.name).read_bytes(), executable())
            self.assertEqual((out / debug.name).read_bytes(), pdb())
            self.assertEqual(manifest["debug_id"], f"{GUID}-1")
            self.assertNotIn(str(root), (out / "manifest.json").read_text())
            with self.assertRaisesRegex(ValueError, "empty"):
                symbols.collect(exe, debug, out, "a" * 40)

    def test_rejects_guid_and_age_mismatch_before_creating_output(self):
        for debug_data in [pdb(2), pdb()[:512 * 4 + 12] + uuid.UUID(int=0).bytes_le]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                exe, debug, out = root / "jajan.exe", root / "jajan.pdb", root / "output"
                exe.write_bytes(executable())
                debug.write_bytes(debug_data.ljust(512 * 5, b"\0"))
                with self.assertRaisesRegex(ValueError, "mismatch"):
                    symbols.collect(exe, debug, out, "a" * 40)
                self.assertFalse(out.exists())

    def test_rejects_missing_truncated_and_invalid_inputs(self):
        for data in [b"", executable()[:610], b"RSDS" + GUID.bytes_le]:
            with self.assertRaises(ValueError):
                symbols.pe_identity(data)
        for data in [b"", pdb()[:512], bytes(4096)]:
            with self.assertRaises(ValueError):
                symbols.pdb_identity(data)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "Git SHA"):
                symbols.collect(root / "exe", root / "pdb", root / "out", "HEAD")


if __name__ == "__main__":
    unittest.main()
