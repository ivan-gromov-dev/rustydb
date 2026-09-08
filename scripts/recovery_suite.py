#!/usr/bin/env python3
"""Exercise snapshot and AOF recovery through real RustyDB processes."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from redis_differential import Client, HOST, available_port, wait_for_server


ROOT = Path(__file__).resolve().parent.parent


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rustydb-binary", type=Path)
    return parser.parse_args()


def resolve_binary(value: Path | None) -> Path:
    if value is not None:
        return value.resolve()
    subprocess.run(
        ["cargo", "build", "--quiet", "--bin", "rustydb"],
        cwd=ROOT,
        check=True,
    )
    name = "rustydb.exe" if os.name == "nt" else "rustydb"
    return (ROOT / "target" / "debug" / name).resolve()


def start(binary: Path, *options: str) -> tuple[subprocess.Popen[bytes], Client]:
    port = available_port()
    process = subprocess.Popen(
        [str(binary), "server", f"{HOST}:{port}", *options],
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    try:
        wait_for_server(HOST, port, process)
        return process, Client(HOST, port)
    except Exception:
        stop(process)
        raise


def stop(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is None:
        process.kill()
    process.wait(timeout=5)


def expect(actual: object, expected: object, label: str) -> None:
    if actual != expected:
        raise AssertionError(f"{label}: returned {actual!r}; expected {expected!r}")


def populate(client: Client) -> None:
    commands = (
        (b"SET", b"string", b"binary\x00value"),
        (b"RPUSH", b"list", b"one", b"two"),
        (b"SADD", b"set", b"alpha", b"beta"),
        (b"HSET", b"hash", b"field", b"value"),
        (b"ZADD", b"zset", b"1.5", b"member"),
        (b"SET", b"ttl", b"alive", b"PX", b"60000"),
    )
    for command in commands:
        result = client.command(*command)
        if isinstance(result, tuple) and result[0] == "error":
            raise AssertionError(f"populate {command[0]!r}: {result[1]!r}")


def verify_populated(client: Client, label: str) -> None:
    checks = (
        ((b"GET", b"string"), b"binary\x00value"),
        ((b"LRANGE", b"list", b"0", b"-1"), [b"one", b"two"]),
        ((b"SISMEMBER", b"set", b"alpha"), 1),
        ((b"SISMEMBER", b"set", b"beta"), 1),
        ((b"HGET", b"hash", b"field"), b"value"),
        ((b"ZSCORE", b"zset", b"member"), b"1.5"),
        ((b"GET", b"ttl"), b"alive"),
    )
    for command, expected in checks:
        expect(client.command(*command), expected, f"{label} {command[0].decode()}")
    pttl = client.command(b"PTTL", b"ttl")
    if not isinstance(pttl, int) or not 0 < pttl <= 60000:
        raise AssertionError(f"{label} PTTL: returned {pttl!r}")


def snapshot_round_trip(binary: Path, directory: Path) -> None:
    path = directory / "state.snapshot"
    process, client = start(binary, "--snapshot", str(path))
    try:
        populate(client)
        expect(client.command(b"SAVE"), b"OK", "snapshot SAVE")
    finally:
        client.close()
        stop(process)

    process, client = start(binary, "--snapshot", str(path))
    try:
        verify_populated(client, "snapshot restart")
    finally:
        client.close()
        stop(process)


def aof_round_trip(binary: Path, directory: Path) -> None:
    path = directory / "state.aof"
    process, client = start(binary, "--aof", str(path))
    try:
        populate(client)
        expect(client.command(b"AOFREWRITE"), b"OK", "AOFREWRITE")
    finally:
        client.close()
        stop(process)

    process, client = start(binary, "--aof", str(path))
    try:
        verify_populated(client, "AOF restart")
    finally:
        client.close()
        stop(process)


def transaction_tail_recovery(binary: Path, directory: Path) -> int:
    path = directory / "transaction.aof"
    process, client = start(binary, "--aof", str(path))
    try:
        expect(client.command(b"SET", b"baseline", b"safe"), b"OK", "baseline SET")
        boundary = path.stat().st_size
        expect(client.command(b"MULTI"), b"OK", "MULTI")
        expect(client.command(b"SET", b"transaction", b"visible"), b"QUEUED", "queued SET")
        expect(
            client.command(b"HSET", b"transaction-hash", b"field", b"value"),
            b"QUEUED",
            "queued HSET",
        )
        expect(client.command(b"EXEC"), [b"OK", 1], "EXEC")
        complete = path.read_bytes()
    finally:
        client.close()
        stop(process)

    if boundary >= len(complete):
        raise AssertionError("EXEC did not append a transaction record")

    process, client = start(binary, "--aof", str(path))
    try:
        expect(client.command(b"GET", b"transaction"), b"visible", "complete transaction")
        expect(
            client.command(b"HGET", b"transaction-hash", b"field"),
            b"value",
            "complete transaction hash",
        )
    finally:
        client.close()
        stop(process)

    partials = 0
    for cut in range(boundary + 1, len(complete)):
        path.write_bytes(complete[:cut])
        process, client = start(binary, "--aof", str(path))
        try:
            expect(client.command(b"GET", b"baseline"), b"safe", f"partial {cut} baseline")
            expect(client.command(b"GET", b"transaction"), None, f"partial {cut} transaction")
            expect(
                client.command(b"HGET", b"transaction-hash", b"field"),
                None,
                f"partial {cut} transaction hash",
            )
        finally:
            client.close()
            stop(process)
        partials += 1

    corrupt = bytearray(complete)
    corrupt[-1] ^= 0xFF
    path.write_bytes(corrupt)
    result = subprocess.run(
        [str(binary), "server", f"{HOST}:{available_port()}", "--aof", str(path)],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=5,
    )
    if result.returncode == 0 or b"checksum" not in result.stderr.lower():
        raise AssertionError(
            "complete checksum corruption did not fail startup clearly: "
            f"code={result.returncode}, stderr={result.stderr!r}"
        )
    return partials


def max_keys_replay(binary: Path, directory: Path) -> None:
    path = directory / "limited.aof"
    process, client = start(binary, "--aof", str(path), "--max-keys", "2")
    try:
        expect(client.command(b"SET", b"a", b"evicted"), b"OK", "limited SET a")
        expect(client.command(b"SET", b"b", b"kept"), b"OK", "limited SET b")
        expect(client.command(b"SET", b"c", b"kept"), b"OK", "limited SET c")
    finally:
        client.close()
        stop(process)

    process, client = start(binary, "--aof", str(path), "--max-keys", "2")
    try:
        expect(client.command(b"GET", b"a"), None, "limited restart evicted key")
        expect(client.command(b"MGET", b"b", b"c"), [b"kept", b"kept"], "limited restart")
    finally:
        client.close()
        stop(process)


def persistence_failures(binary: Path, directory: Path) -> None:
    missing_snapshot = directory / "missing" / "state.snapshot"
    process, client = start(binary, "--snapshot", str(missing_snapshot))
    try:
        expect(client.command(b"SET", b"live", b"value"), b"OK", "failed SAVE SET")
        result = client.command(b"SAVE")
        if not isinstance(result, tuple) or result[0] != "error":
            raise AssertionError(f"SAVE to a missing directory returned {result!r}")
        expect(client.command(b"GET", b"live"), b"value", "server after failed SAVE")
    finally:
        client.close()
        stop(process)

    invalid_aof = directory / "absent" / "state.aof"
    result = subprocess.run(
        [str(binary), "server", f"{HOST}:{available_port()}", "--aof", str(invalid_aof)],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=5,
    )
    if result.returncode == 0 or b"AOF" not in result.stderr:
        raise AssertionError(
            "AOF open failure was not reported clearly: "
            f"code={result.returncode}, stderr={result.stderr!r}"
        )


def multi_client_aof_recovery(binary: Path, directory: Path) -> None:
    path = directory / "multi-client.aof"
    process, first = start(binary, "--aof", str(path))
    second = Client(HOST, first.socket.getpeername()[1])
    try:
        expect(first.command(b"SET", b"first", b"one"), b"OK", "client one SET")
        expect(second.command(b"RPUSH", b"shared", b"a", b"b"), 2, "client two RPUSH")
        expect(first.command(b"MULTI"), b"OK", "client one MULTI")
        expect(first.command(b"HSET", b"hash", b"field", b"value"), b"QUEUED", "queued HSET")
        expect(first.command(b"ZADD", b"scores", b"2", b"member"), b"QUEUED", "queued ZADD")
        expect(first.command(b"EXEC"), [1, 1], "client one EXEC")
        expect(second.command(b"SADD", b"set", b"member"), 1, "client two SADD")
    finally:
        first.close()
        second.close()
        stop(process)

    process, client = start(binary, "--aof", str(path))
    try:
        checks = (
            ((b"GET", b"first"), b"one"),
            ((b"LRANGE", b"shared", b"0", b"-1"), [b"a", b"b"]),
            ((b"HGET", b"hash", b"field"), b"value"),
            ((b"ZSCORE", b"scores", b"member"), b"2"),
            ((b"SISMEMBER", b"set", b"member"), 1),
        )
        for command, expected in checks:
            expect(client.command(*command), expected, f"multi-client restart {command[0]!r}")
    finally:
        client.close()
        stop(process)

def main() -> int:
    binary = resolve_binary(arguments().rustydb_binary)
    with tempfile.TemporaryDirectory(prefix="rustydb-recovery-") as temporary:
        directory = Path(temporary)
        snapshot_round_trip(binary, directory)
        aof_round_trip(binary, directory)
        partials = transaction_tail_recovery(binary, directory)
        max_keys_replay(binary, directory)
        persistence_failures(binary, directory)
        multi_client_aof_recovery(binary, directory)
    print(
        "recovery suite passed: snapshot, AOF rewrite, all value types, TTL, "
        "max-keys, persistence failures, multi-client replay, checksum failure, "
        f"and {partials} transaction-tail truncations"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"recovery suite failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
