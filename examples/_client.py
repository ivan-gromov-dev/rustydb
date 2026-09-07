"""Minimal RESP2 client and disposable local server for the examples."""
from contextlib import contextmanager
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time


class Client:
    def __init__(self, connection):
        self.connection = connection
        self.reader = connection.makefile("rb")

    def command(self, *args):
        args = [value if isinstance(value, bytes) else str(value).encode() for value in args]
        request = f"*{len(args)}\r\n".encode()
        for value in args:
            request += f"${len(value)}\r\n".encode() + value + b"\r\n"
        self.connection.sendall(request)
        return self.read()

    def read(self):
        line = self.reader.readline()
        if not line.endswith(b"\r\n"):
            raise RuntimeError("Truncated RESP response")
        kind, value = line[:1], line[1:-2]
        if kind == b"-":
            raise RuntimeError(value.decode(errors="replace"))
        if kind == b"+":
            return value
        if kind == b":":
            return int(value)
        if kind == b"*":
            return [self.read() for _ in range(int(value))]
        if kind == b"$":
            length = int(value)
            if length == -1:
                return None
            data = self.reader.read(length + 2)
            if len(data) != length + 2 or data[-2:] != b"\r\n":
                raise RuntimeError("Truncated bulk string")
            return data[:-2]
        raise RuntimeError(f"Unexpected RESP type: {kind!r}")


@contextmanager
def demo_server():
    root = Path(__file__).resolve().parents[1]
    binary = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "target" / "debug" / ("rustydb.exe" if os.name == "nt" else "rustydb")
    if not binary.is_file():
        raise SystemExit("Build first: cargo build --bin rustydb (or pass the binary path)")
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    with tempfile.TemporaryDirectory(prefix="rustydb-example-") as directory:
        process = subprocess.Popen([str(binary), "server", f"127.0.0.1:{port}"], cwd=directory,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        connection = None
        try:
            deadline = time.monotonic() + 5
            while connection is None:
                if process.poll() is not None:
                    raise RuntimeError(process.stderr.read().decode(errors="replace"))
                try:
                    connection = socket.create_connection(("127.0.0.1", port), timeout=1)
                except OSError:
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Example server did not start")
                    time.sleep(0.02)
            client = Client(connection)
            try:
                yield client
            finally:
                client.reader.close()
        finally:
            if connection is not None:
                connection.close()
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)
            process.stderr.close()
