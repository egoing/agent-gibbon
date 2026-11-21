"""Experimental local single-session PTY owner and disposable foreground client."""
import errno
import fcntl
import os
import pathlib
import pty
import re
import select
import signal
import socket
import stat
import subprocess
import sys
import termios
import time
import tty

mode, directory, session, shell = sys.argv[1:]
root = pathlib.Path(directory)
if not re.fullmatch(r"terminal-[0-9]{1,48}", session):
    raise SystemExit("invalid session")
info = root.lstat()
if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
    raise SystemExit("private directory required")
endpoint = str(root / session)
lease = max(1, min(900, int(os.environ.get("JAJAN_POC_DETACH_SECONDS", "300"))))
os.umask(0o077)


def event(name):
    # Keep terminal traffic and exception text out of the lifecycle log.
    try:
        with (root / "lifecycle.log").open("a", encoding="ascii") as log:
            log.write(f"{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())} {session} {name}\n")
    except OSError:
        pass


def owner():
    guard = open(root / (session + ".lock"), "a")
    try:
        fcntl.flock(guard, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return
    listener = socket.socket(socket.AF_UNIX)
    pathlib.Path(endpoint).unlink(missing_ok=True)
    listener.bind(endpoint)
    listener.listen(4)
    peer = None
    child = master = None
    deadline = time.monotonic() + 3
    pending = bytearray()
    outgoing = bytearray()
    try:
        while True:
            if child and os.waitpid(child, os.WNOHANG)[0]:
                event("shell-exited")
                child = None
                break
            if peer is None and time.monotonic() > deadline:
                event("lease-expired" if child else "initial-attach-timeout")
                break
            reads = [listener]
            writes = []
            if master is not None:
                reads.append(master)
                if pending:
                    writes.append(master)
            if peer is not None:
                if len(pending) < 65536:
                    reads.append(peer)
                if outgoing:
                    writes.append(peer)
            readable, writable, _ = select.select(reads, writes, [], 0.05)
            if listener in readable:
                conn, _ = listener.accept()
                if peer is not None:
                    event("duplicate-rejected")
                    conn.close()
                else:
                    reconnecting = child is not None
                    if child is None:
                        initial_line = os.environ.pop("JAJAN_POC_INITIAL_LINE", "")
                        child, master = pty.fork()
                        if child == 0:
                            os.execv(shell, [shell, "-i"])
                        event("shell-created")
                        os.set_blocking(master, False)
                        if os.environ.get("JAJAN_SHELL_SETUP"):
                            pending.extend(b' eval "$JAJAN_SHELL_SETUP"; unset JAJAN_SHELL_SETUP\r')
                        if initial_line:
                            pending.extend(initial_line.encode() + b"\r")
                    conn.sendall(b"1")
                    if reconnecting:
                        event("reattached")
                    conn.setblocking(False)
                    peer = conn
            if master in readable:
                try:
                    chunk = os.read(master, 8192)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
                    event("pty-closed")
                    break
                if not chunk:
                    event("pty-closed")
                    break
                outgoing.extend(chunk)
                del outgoing[:-65536]
            if master in writable:
                del pending[:os.write(master, pending)]
            if peer is not None:
                try:
                    if peer in readable:
                        data = peer.recv(8192)
                        if not data:
                            raise ConnectionResetError()
                        pending.extend(data)
                    if peer in writable:
                        del outgoing[:peer.send(outgoing)]
                except (BrokenPipeError, ConnectionResetError):
                    peer.close()
                    peer = None
                    pending.clear()
                    event("detached")
                    deadline = time.monotonic() + lease
    finally:
        if peer is not None:
            peer.close()
        if master is not None:
            os.close(master)
        if child:
            for sig in (signal.SIGHUP, signal.SIGKILL):
                try:
                    os.kill(child, sig)
                except ProcessLookupError:
                    pass
                time.sleep(0.05)
                if os.waitpid(child, os.WNOHANG)[0]:
                    break
            else:
                os.waitpid(child, 0)
        listener.close()
        pathlib.Path(endpoint).unlink(missing_ok=True)
        guard.close()


def client():
    deadline = time.monotonic() + 2
    launched = False
    while True:
        conn = socket.socket(socket.AF_UNIX)
        try:
            conn.settimeout(max(0.01, deadline - time.monotonic()))
            conn.connect(endpoint)
            if conn.recv(1) != b"1":
                raise RuntimeError("session already attached")
            break
        except socket.timeout:
            event("attach-timeout")
            conn.close()
            raise
        except (FileNotFoundError, ConnectionRefusedError):
            conn.close()
            if time.monotonic() >= deadline:
                event("attach-timeout")
                raise TimeoutError("PTY owner unavailable")
            if not launched:
                subprocess.Popen([sys.executable, __file__, "owner", directory, session, shell],
                                 stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.DEVNULL, start_new_session=True, close_fds=True)
                launched = True
            time.sleep(0.025)
    conn.settimeout(None)
    saved = termios.tcgetattr(0)
    try:
        tty.setraw(0, termios.TCSANOW)
        while True:
            ready, _, _ = select.select([0, conn], [], [])
            if conn in ready:
                data = conn.recv(8192)
                if not data:
                    break
                while data:
                    data = data[os.write(1, data):]
            if 0 in ready:
                data = os.read(0, 8192)
                if not data:
                    break
                conn.sendall(data)
    finally:
        conn.close()
        termios.tcsetattr(0, termios.TCSANOW, saved)


if mode == "client":
    client()
elif mode == "owner":
    owner()
else:
    raise SystemExit("unknown mode")
