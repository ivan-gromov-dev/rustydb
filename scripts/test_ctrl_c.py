#!/usr/bin/env python3
"""Exercise the server's graceful shutdown through a real SIGINT."""

from __future__ import annotations

import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time


def reserve_address() -> tuple[str, int]:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()


def connect_when_ready(
    process: subprocess.Popen[bytes], address: tuple[str, int]
) -> socket.socket:
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if process.poll() is not None:
            stdout, stderr = process.communicate()
            raise RuntimeError(
                "server exited before accepting connections\n"
                f"stdout: {stdout.decode(errors='replace')}\n"
                f"stderr: {stderr.decode(errors='replace')}"
            )
        try:
            return socket.create_connection(address, timeout=0.2)
        except OSError:
            time.sleep(0.05)
    raise TimeoutError(f"server did not listen on {address[0]}:{address[1]}")


def wait_until_connections_stop(address: tuple[str, int]) -> None:
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        try:
            with socket.create_connection(address, timeout=0.2):
                time.sleep(0.05)
        except OSError:
            return
    raise TimeoutError("server continued accepting connections after SIGINT")


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} RUSTYDB_BINARY", file=sys.stderr)
        return 2
    if os.name != "posix":
        print("this test requires POSIX signal support", file=sys.stderr)
        return 2

    binary = Path(sys.argv[1]).resolve()
    address = reserve_address()
    process = subprocess.Popen(
        [str(binary), "server", f"{address[0]}:{address[1]}"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    client: socket.socket | None = None
    stream = None

    try:
        client = connect_when_ready(process, address)
        client.settimeout(5)
        stream = client.makefile("rwb")
        stream.write(b"SET shutdown-test value\n")
        stream.flush()
        assert stream.readline() == b"OK\n"

        process.send_signal(signal.SIGINT)
        wait_until_connections_stop(address)

        stream.write(b"GET shutdown-test\nEXIT\n")
        stream.flush()
        assert stream.readline() == b"value\n"
        assert stream.readline() == b"Bye!\n"
        stream.close()
        stream = None
        client.close()
        client = None

        stdout, stderr = process.communicate(timeout=5)
        assert process.returncode == 0, f"server exited with {process.returncode}"
        assert stdout == b"", f"unexpected stdout: {stdout!r}"
        assert stderr == b"", f"unexpected stderr: {stderr!r}"
    finally:
        if stream is not None:
            stream.close()
        if client is not None:
            client.close()
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=2)

    print("Ctrl+C graceful shutdown test passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
