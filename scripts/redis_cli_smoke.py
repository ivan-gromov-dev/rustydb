#!/usr/bin/env python3
"""Run a small compatibility smoke test against an installed redis-cli."""

from __future__ import annotations

import os
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
HOST = "127.0.0.1"
BINARY_VALUE = b"line one\nline two\x00tail"


def available_port() -> int:
    with socket.socket() as listener:
        listener.bind((HOST, 0))
        return int(listener.getsockname()[1])


def wait_for_server(port: int, process: subprocess.Popen[bytes]) -> None:
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("RustyDB server stopped before accepting connections")
        try:
            with socket.create_connection((HOST, port), timeout=0.1):
                return
        except OSError:
            time.sleep(0.05)
    raise RuntimeError("RustyDB server did not start within five seconds")


def redis(
    cli: str,
    port: int,
    *arguments: str,
    stdin: bytes | None = None,
    protocol: int = 2,
) -> bytes:
    result = subprocess.run(
        [cli, f"-{protocol}", "--raw", "-h", HOST, "-p", str(port), *arguments],
        cwd=ROOT,
        input=stdin,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=5,
    )
    if result.returncode != 0:
        detail = result.stderr.decode(errors="replace").strip()
        raise RuntimeError(f"redis-cli {' '.join(arguments)} failed: {detail}")
    return result.stdout


def line(output: bytes) -> bytes:
    if output.endswith(b"\r\n"):
        return output[:-2]
    if output.endswith(b"\n"):
        return output[:-1]
    return output


def expect(actual: bytes, expected: bytes, command: str) -> None:
    if actual != expected:
        raise RuntimeError(
            f"{command} returned {actual!r}; expected {expected!r}"
        )


def main() -> int:
    cli = os.environ.get("RUSTYDB_REDIS_CLI") or shutil.which("redis-cli")
    if cli is None:
        print(
            "redis-cli was not found; install it or set RUSTYDB_REDIS_CLI",
            file=sys.stderr,
        )
        return 2

    subprocess.run(["cargo", "build", "--quiet"], cwd=ROOT, check=True)
    executable = ROOT / "target" / "debug" / (
        "rustydb.exe" if os.name == "nt" else "rustydb"
    )
    port = available_port()
    server = subprocess.Popen(
        [str(executable), "server", f"{HOST}:{port}"],
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )

    try:
        wait_for_server(port, server)
        expect(line(redis(cli, port, "PING")), b"PONG", "PING")
        expect(
            line(redis(cli, port, "PING", protocol=3)),
            b"PONG",
            "RESP3 PING",
        )
        client_id = line(redis(cli, port, "CLIENT", "ID"))
        if not client_id.isdigit() or int(client_id) <= 0:
            raise AssertionError(f"CLIENT ID: expected a positive integer, got {client_id!r}")
        expect(
            line(redis(cli, port, "CLIENT", "SETINFO", "LIB-NAME", "rustydb-smoke")),
            b"OK",
            "CLIENT SETINFO",
        )
        command_count = line(redis(cli, port, "COMMAND", "COUNT"))
        if not command_count.isdigit() or int(command_count) <= 0:
            raise AssertionError(
                f"COMMAND COUNT: expected a positive integer, got {command_count!r}"
            )
        expect(
            line(redis(cli, port, "PING", "hello world")),
            b"hello world",
            "PING message",
        )
        expect(
            line(redis(cli, port, "-x", "ECHO", stdin=BINARY_VALUE)),
            BINARY_VALUE,
            "binary ECHO",
        )
        expect(line(redis(cli, port, "SET", "greeting", "hello world")), b"OK", "SET")
        expect(line(redis(cli, port, "GET", "greeting")), b"hello world", "GET")
        expect(line(redis(cli, port, "INCR", "counter")), b"1", "INCR")
        expect(line(redis(cli, port, "RPUSH", "items", "second")), b"1", "RPUSH")
        expect(line(redis(cli, port, "LPUSH", "items", "first")), b"2", "LPUSH")
        expect(redis(cli, port, "LRANGE", "items", "0", "-1"), b"first\nsecond\n", "LRANGE")
        expect(line(redis(cli, port, "SADD", "letters", "b")), b"1", "SADD")
        expect(line(redis(cli, port, "SADD", "letters", "a")), b"1", "SADD")
        expect(redis(cli, port, "SMEMBERS", "letters"), b"a\nb\n", "SMEMBERS")
        expect(line(redis(cli, port, "TTL", "greeting")), b"-1", "TTL")
        expect(
            line(redis(cli, port, "-x", "SET", "binary", stdin=BINARY_VALUE)),
            b"OK",
            "binary SET",
        )
        expect(line(redis(cli, port, "GET", "binary")), BINARY_VALUE, "binary GET")
    finally:
        if server.poll() is None:
            server.kill()
        server.wait(timeout=5)

    print("redis-cli RESP2/RESP3 smoke test passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
