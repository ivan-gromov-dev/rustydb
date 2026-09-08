#!/usr/bin/env python3
"""Run a deterministic model-based workload and verify AOF crash recovery."""

from __future__ import annotations

import argparse
import random
import subprocess
import sys
import tempfile
from pathlib import Path

from recovery_suite import Client, expect, resolve_binary, start, stop


DEFAULT_SEED = 1_592_594_996
KEY_COUNT = 8
ITEM_COUNT = 10


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rustydb-binary", type=Path)
    parser.add_argument("--operations", type=int, default=5_000)
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED)
    args = parser.parse_args()
    if args.operations < 1:
        parser.error("--operations must be positive")
    return args


def keys(prefix: bytes) -> list[bytes]:
    return [prefix + b":" + bytes([index, 0x00, 0xFF]) for index in range(KEY_COUNT)]


def items(prefix: bytes) -> list[bytes]:
    return [prefix + b":" + bytes([index, 0x00, 0xFE]) for index in range(ITEM_COUNT)]


def verify_state(client: Client, model: dict[str, dict[bytes, object]], label: str) -> None:
    string_keys = keys(b"string")
    list_keys = keys(b"list")
    set_keys = keys(b"set")
    hash_keys = keys(b"hash")
    zset_keys = keys(b"zset")

    for key in string_keys:
        expect(client.command(b"GET", key), model["strings"].get(key), f"{label} GET")
    for key in list_keys:
        expected = model["lists"].get(key)
        if expected is None:
            expect(client.command(b"TYPE", key), b"none", f"{label} missing list")
        else:
            expect(client.command(b"LRANGE", key, b"0", b"-1"), expected, f"{label} LRANGE")
    for key in set_keys:
        expected = model["sets"].get(key)
        if expected is None:
            expect(client.command(b"TYPE", key), b"none", f"{label} missing set")
        else:
            expect(client.command(b"SMEMBERS", key), sorted(expected), f"{label} SMEMBERS")
    for key in hash_keys:
        expected = model["hashes"].get(key)
        if expected is None:
            expect(client.command(b"TYPE", key), b"none", f"{label} missing hash")
        else:
            flattened = [part for field in sorted(expected) for part in (field, expected[field])]
            expect(client.command(b"HGETALL", key), flattened, f"{label} HGETALL")
    for key in zset_keys:
        expected = model["zsets"].get(key)
        if expected is None:
            expect(client.command(b"TYPE", key), b"none", f"{label} missing zset")
        else:
            ordered = sorted(expected.items(), key=lambda entry: (entry[1], entry[0]))
            flattened = [part for member, score in ordered for part in (member, str(score).encode())]
            expect(
                client.command(b"ZRANGE", key, b"0", b"-1", b"WITHSCORES"),
                flattened,
                f"{label} ZRANGE",
            )

    live = sum(len(values) for values in model.values())
    expect(client.command(b"DBSIZE"), live, f"{label} DBSIZE")


def run_operation(
    client: Client,
    model: dict[str, dict[bytes, object]],
    rng: random.Random,
    index: int,
) -> None:
    family = rng.randrange(5)
    delete = rng.randrange(7) == 0
    label = f"operation {index}"

    if family == 0:
        key = rng.choice(keys(b"string"))
        strings = model["strings"]
        if delete:
            expected = int(key in strings)
            strings.pop(key, None)
            expect(client.command(b"DEL", key), expected, f"{label} DEL string")
        else:
            value = rng.choice(items(b"value"))
            strings[key] = value
            expect(client.command(b"SET", key, value), b"OK", f"{label} SET")
    elif family == 1:
        key = rng.choice(keys(b"list"))
        lists = model["lists"]
        if delete:
            expected = int(key in lists)
            lists.pop(key, None)
            expect(client.command(b"DEL", key), expected, f"{label} DEL list")
        elif rng.randrange(3) == 0:
            values = lists.get(key)
            expected = None if values is None else values.pop(0)
            if values == []:
                lists.pop(key)
            expect(client.command(b"LPOP", key), expected, f"{label} LPOP")
        else:
            value = rng.choice(items(b"list-value"))
            values = lists.setdefault(key, [])
            values.append(value)
            expect(client.command(b"RPUSH", key, value), len(values), f"{label} RPUSH")
    elif family == 2:
        key = rng.choice(keys(b"set"))
        sets = model["sets"]
        if delete:
            expected = int(key in sets)
            sets.pop(key, None)
            expect(client.command(b"DEL", key), expected, f"{label} DEL set")
        else:
            member = rng.choice(items(b"set-member"))
            values = sets.get(key)
            if rng.randrange(3) == 0:
                expected = int(values is not None and member in values)
                if expected:
                    values.remove(member)
                    if not values:
                        sets.pop(key)
                expect(client.command(b"SREM", key, member), expected, f"{label} SREM")
            else:
                if values is None:
                    values = set()
                    sets[key] = values
                expected = int(member not in values)
                values.add(member)
                expect(client.command(b"SADD", key, member), expected, f"{label} SADD")
    elif family == 3:
        key = rng.choice(keys(b"hash"))
        hashes = model["hashes"]
        if delete:
            expected = int(key in hashes)
            hashes.pop(key, None)
            expect(client.command(b"DEL", key), expected, f"{label} DEL hash")
        else:
            field = rng.choice(items(b"field"))
            values = hashes.get(key)
            if rng.randrange(3) == 0:
                expected = int(values is not None and field in values)
                if expected:
                    del values[field]
                    if not values:
                        hashes.pop(key)
                expect(client.command(b"HDEL", key, field), expected, f"{label} HDEL")
            else:
                if values is None:
                    values = {}
                    hashes[key] = values
                expected = int(field not in values)
                values[field] = rng.choice(items(b"hash-value"))
                expect(
                    client.command(b"HSET", key, field, values[field]),
                    expected,
                    f"{label} HSET",
                )
    else:
        key = rng.choice(keys(b"zset"))
        zsets = model["zsets"]
        if delete:
            expected = int(key in zsets)
            zsets.pop(key, None)
            expect(client.command(b"DEL", key), expected, f"{label} DEL zset")
        else:
            member = rng.choice(items(b"zset-member"))
            values = zsets.get(key)
            if rng.randrange(3) == 0:
                expected = int(values is not None and member in values)
                if expected:
                    del values[member]
                    if not values:
                        zsets.pop(key)
                expect(client.command(b"ZREM", key, member), expected, f"{label} ZREM")
            else:
                if values is None:
                    values = {}
                    zsets[key] = values
                expected = int(member not in values)
                score = rng.randint(-1_000, 1_000)
                values[member] = score
                expect(
                    client.command(b"ZADD", key, str(score).encode(), member),
                    expected,
                    f"{label} ZADD",
                )


def workload(binary: Path, operations: int, seed: int, directory: Path) -> int:
    path = directory / "randomized.aof"
    model: dict[str, dict[bytes, object]] = {
        "strings": {},
        "lists": {},
        "sets": {},
        "hashes": {},
        "zsets": {},
    }
    rng = random.Random(seed)
    process, client = start(binary, "--aof", str(path))
    try:
        for index in range(operations):
            run_operation(client, model, rng, index)
            if (index + 1) % 500 == 0:
                verify_state(client, model, f"checkpoint {index + 1}")
        verify_state(client, model, "before crash")
    finally:
        client.close()
        stop(process)

    process, client = start(binary, "--aof", str(path))
    try:
        verify_state(client, model, "after restart")
    finally:
        client.close()
        stop(process)
    return path.stat().st_size


def main() -> int:
    args = arguments()
    binary = resolve_binary(args.rustydb_binary)
    with tempfile.TemporaryDirectory(prefix="rustydb-randomized-recovery-") as temporary:
        aof_size = workload(binary, args.operations, args.seed, Path(temporary))
    print(
        "randomized recovery passed: "
        f"operations={args.operations}, seed={args.seed}, aof_bytes={aof_size}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"randomized recovery failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
