#!/usr/bin/env python3
"""Compare representative RustyDB commands with a pinned Redis server."""

from __future__ import annotations

import argparse
import os
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import TypeAlias


ROOT = Path(__file__).resolve().parent.parent
HOST = "127.0.0.1"
Resp: TypeAlias = bytes | int | None | list["Resp"] | tuple[str, bytes]


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--redis-host", default=HOST)
    parser.add_argument("--redis-port", type=int, default=6380)
    parser.add_argument("--rustydb-binary", type=Path)
    return parser.parse_args()


def available_port() -> int:
    with socket.socket() as listener:
        listener.bind((HOST, 0))
        return int(listener.getsockname()[1])


def wait_for_server(host: str, port: int, process: subprocess.Popen[bytes] | None) -> None:
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if process is not None and process.poll() is not None:
            raise RuntimeError("RustyDB stopped before accepting connections")
        try:
            with socket.create_connection((host, port), timeout=0.2):
                return
        except OSError:
            time.sleep(0.05)
    raise RuntimeError(f"server at {host}:{port} did not become ready")


def encode(parts: tuple[bytes, ...]) -> bytes:
    request = bytearray(f"*{len(parts)}\r\n".encode())
    for part in parts:
        request.extend(f"${len(part)}\r\n".encode())
        request.extend(part)
        request.extend(b"\r\n")
    return bytes(request)


class Client:
    def __init__(self, host: str, port: int) -> None:
        self.socket = socket.create_connection((host, port), timeout=5)
        self.reader = self.socket.makefile("rb")

    def close(self) -> None:
        self.reader.close()
        self.socket.close()

    def command(self, *parts: bytes) -> Resp:
        self.socket.sendall(encode(parts))
        return self.response()

    def line(self) -> bytes:
        line = self.reader.readline()
        if not line.endswith(b"\r\n"):
            raise RuntimeError(f"truncated RESP line: {line!r}")
        return line[:-2]

    def response(self) -> Resp:
        prefix = self.reader.read(1)
        if prefix == b"+":
            return self.line()
        if prefix == b"-":
            return ("error", self.line())
        if prefix == b":":
            return int(self.line())
        if prefix == b"$":
            length = int(self.line())
            if length == -1:
                return None
            value = self.reader.read(length)
            if len(value) != length or self.reader.read(2) != b"\r\n":
                raise RuntimeError("truncated RESP bulk string")
            return value
        if prefix == b"*":
            length = int(self.line())
            if length == -1:
                return None
            return [self.response() for _ in range(length)]
        if prefix == b"_":
            if self.line():
                raise RuntimeError("malformed RESP3 null")
            return None
        if prefix == b"#":
            value = self.line()
            return 1 if value == b"t" else 0
        if prefix == b",":
            return self.line()
        if prefix in (b"%", b">"):
            length = int(self.line())
            width = length * 2 if prefix == b"%" else length
            return [self.response() for _ in range(width)]
        raise RuntimeError(f"unsupported RESP prefix: {prefix!r}")


Scenario = tuple[str, tuple[tuple[bytes, ...], ...]]


SCENARIOS: tuple[Scenario, ...] = (
    ("connection", ((b"PING",), (b"ECHO", b"binary\x00value"))),
    (
        "strings",
        (
            (b"SET", b"string", b"10"),
            (b"GET", b"string"),
            (b"APPEND", b"string", b"5"),
            (b"STRLEN", b"string"),
            (b"INCRBY", b"string", b"2"),
            (b"MSET", b"one", b"1", b"two", b"2"),
            (b"MGET", b"two", b"missing", b"one"),
        ),
    ),
    (
        "keys and expiry",
        (
            (b"SET", b"expiring", b"value"),
            (b"TTL", b"expiring"),
            (b"EXPIRE", b"expiring", b"60"),
            (b"PERSIST", b"expiring"),
            (b"TYPE", b"expiring"),
            (b"EXISTS", b"expiring", b"missing"),
            (b"DEL", b"expiring"),
        ),
    ),
    (
        "lists",
        (
            (b"RPUSH", b"list", b"b", b"c"),
            (b"LPUSH", b"list", b"a"),
            (b"LRANGE", b"list", b"0", b"-1"),
            (b"LINDEX", b"list", b"-1"),
            (b"LPOP", b"list"),
            (b"LLEN", b"list"),
        ),
    ),
    (
        "sets",
        (
            (b"SADD", b"set", b"a", b"b"),
            (b"SISMEMBER", b"set", b"b"),
            (b"SMISMEMBER", b"set", b"b", b"missing"),
            (b"SCARD", b"set"),
            (b"SREM", b"set", b"a"),
        ),
    ),
    (
        "hashes",
        (
            (b"HSET", b"hash", b"one", b"1", b"two", b"2"),
            (b"HGET", b"hash", b"one"),
            (b"HMGET", b"hash", b"two", b"missing", b"one"),
            (b"HINCRBY", b"hash", b"one", b"2"),
            (b"HLEN", b"hash"),
        ),
    ),
    (
        "sorted sets",
        (
            (b"ZADD", b"zset", b"2", b"b", b"1", b"a"),
            (b"ZSCORE", b"zset", b"a"),
            (b"ZRANGE", b"zset", b"0", b"-1", b"WITHSCORES"),
            (b"ZRANK", b"zset", b"b"),
            (b"ZCOUNT", b"zset", b"(1", b"+inf"),
        ),
    ),
    (
        "transactions",
        (
            (b"MULTI",),
            (b"SET", b"transaction", b"value"),
            (b"GET", b"transaction"),
            (b"EXEC",),
        ),
    ),
    (
        "pubsub",
        (
            (b"PUBLISH", b"unused", b"message"),
            (b"PUBSUB", b"NUMSUB", b"unused"),
        ),
    ),
    (
        "database",
        ((b"DBSIZE",), (b"SELECT", b"0"), (b"FLUSHDB", b"SYNC"), (b"DBSIZE",)),
    ),
)


def compare(redis: Client, rustydb: Client) -> int:
    compared = 0
    for name, commands in SCENARIOS:
        redis.command(b"FLUSHALL")
        rustydb.command(b"FLUSHALL")
        for command in commands:
            expected = redis.command(*command)
            actual = rustydb.command(*command)
            if actual != expected:
                rendered = b" ".join(command).decode(errors="backslashreplace")
                raise AssertionError(
                    f"{name}: {rendered}: RustyDB returned {actual!r}; Redis returned {expected!r}"
                )
            compared += 1
    return compared


def main() -> int:
    args = arguments()
    binary = args.rustydb_binary
    if binary is None:
        subprocess.run(["cargo", "build", "--quiet", "--bin", "rustydb"], cwd=ROOT, check=True)
        binary = ROOT / "target" / "debug" / ("rustydb.exe" if os.name == "nt" else "rustydb")

    wait_for_server(args.redis_host, args.redis_port, None)
    rustydb_port = available_port()
    directory = tempfile.TemporaryDirectory(prefix="rustydb-differential-")
    snapshot = Path(directory.name) / "state.snapshot"
    process = subprocess.Popen(
        [str(binary), "server", f"{HOST}:{rustydb_port}", "--snapshot", str(snapshot)],
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    try:
        wait_for_server(HOST, rustydb_port, process)
        redis = Client(args.redis_host, args.redis_port)
        rustydb = Client(HOST, rustydb_port)
        try:
            count = compare(redis, rustydb)
        finally:
            redis.close()
            rustydb.close()
    finally:
        if process.poll() is None:
            process.kill()
        process.wait(timeout=5)
        directory.cleanup()

    print(f"Redis differential suite passed: {count} command results matched")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"differential test failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
