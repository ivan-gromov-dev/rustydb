# Redis Compatibility for RustyDB 1.0

RustyDB implements a focused Redis-compatible command and wire-protocol subset.
The exact accepted forms are authoritative in the command table in
[README.md](README.md). This matrix records the compatibility boundary that the
automated verification compares against a pinned Redis release.

## Matrix

| Area            | Supported subset                                                                                                                                                                               | Intentional differences                                                                                                 |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Protocol        | RESP2 arrays of bulk-string requests; required RESP2 replies; required RESP3 replies and push messages; `HELLO 2` and `HELLO 3`                                                                | No inline protocol; only the response types required by supported commands                                              |
| Connection      | `PING`, `ECHO`, `QUIT`, `CLIENT ID`, `CLIENT SETNAME`, `CLIENT GETNAME`, `CLIENT SETINFO`                                                                                                      | No authentication, tracking, pause, kill, or complete client metadata                                                   |
| Strings         | `SET`, `MSET`, `MSETNX`, `SETNX`, `GET`, `GETEX`, `MGET`, `GETSET`, `GETDEL`, `APPEND`, `STRLEN`, `GETRANGE`, `SETRANGE`, integer increments/decrements, `INCRBYFLOAT`                         | Only the options shown in README; numeric and error text compatibility is limited to documented behavior                |
| Keys and expiry | `EXISTS`, `DEL`, `UNLINK`, `TYPE`, `TOUCH`, `RENAME`, `COPY`, `KEYS`, `SCAN`, `RANDOMKEY`, `EXPIRE`, `PEXPIRE`, `EXPIREAT`, `PEXPIREAT`, `TTL`, `PTTL`, `EXPIRETIME`, `PEXPIRETIME`, `PERSIST` | `UNLINK` is synchronous; deterministic key/member ordering; `COPY` supports database 0 only                             |
| Lists           | Documented push, pop, range, index, insert, trim, remove, position, move, and blocking move/pop forms                                                                                          | Blocking operations use RustyDB's shared-server scheduling; only documented options are accepted                        |
| Sets            | Documented membership, random selection, move, algebra, store, member, cardinality, and scan forms                                                                                             | `SPOP` and other collection results use deterministic binary order rather than Redis randomness or unspecified ordering |
| Hashes          | `HSET`, `HSETNX`, `HGET`, `HMGET`, `HGETALL`, `HDEL`, `HEXISTS`, `HLEN`, `HKEYS`, `HVALS`, `HINCRBY`, `HINCRBYFLOAT`, `HSCAN`                                                                  | Field and scan output is deterministic binary order                                                                     |
| Sorted sets     | Documented add, remove, score, multi-score, increment, cardinality, rank, range, count, pop, range-removal, and scan forms                                                                     | Only documented `ZADD` and `ZRANGE` options; equal-score members use binary order                                       |
| Transactions    | `MULTI`, `EXEC`, `DISCARD`, `WATCH`, `UNWATCH`                                                                                                                                                 | Persistence and the documented queue-time/execution-time model are supported; scripting is not                          |
| Pub/Sub         | `PUBLISH`, `SUBSCRIBE`, `UNSUBSCRIBE`, `PSUBSCRIBE`, `PUNSUBSCRIBE`, `PUBSUB CHANNELS`, `PUBSUB NUMSUB`                                                                                        | In-process, non-persistent delivery only; no shard Pub/Sub or `PUBSUB NUMPAT`                                           |
| Database        | `SELECT 0`, `DBSIZE`, `FLUSHDB`, `FLUSHALL` with documented modifiers                                                                                                                          | Exactly one logical database; flush is synchronous even when `ASYNC` is accepted                                        |
| Metadata        | `COMMAND`, `COMMAND INFO`, `COMMAND COUNT`, `INFO`                                                                                                                                             | Metadata and error text cover RustyDB's subset rather than Redis's complete schema                                      |
| Persistence     | RustyDB `SAVE` and `AOFREWRITE` commands                                                                                                                                                       | Snapshot and AOF formats are RustyDB-specific and are not Redis RDB/AOF compatible                                      |

## Unsupported scope

RustyDB does not claim compatibility for commands or options absent from the
README table. In particular, 1.0 excludes authentication and ACLs, TLS,
multiple databases, scripting and functions, streams, replication, Sentinel,
Cluster, modules, and Redis configuration or persistence file formats.

Interactive-only `HELP`, `CLEAR`, `LEN`, and `EXIT` are RustyDB conveniences and
are not Redis compatibility claims. Redis CLI client-side behavior is also not
part of the server contract.

## Verification status

The `redis-cli` smoke test exercises representative RESP2 and RESP3 commands.
The differential suite compares representative successful RESP2 behavior from
every command family in this matrix with Redis 7.4.1. Command metadata tests
require every advertised command to have a unique registry entry, while parser,
executor, RESP, CLI, and TCP tests cover the exact accepted forms and intentional
differences documented in the README. Compatibility outside those forms is not
claimed.
