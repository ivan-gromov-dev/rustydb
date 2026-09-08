"""Atomically update related keys with optimistic locking."""

from _client import demo_server


def main():
    with demo_server() as client:
        assert client.command("SET", "account:alice", 10) == b"OK"
        assert client.command("WATCH", "account:alice") == b"OK"
        assert client.command("MULTI") == b"OK"
        assert client.command("DECRBY", "account:alice", 3) == b"QUEUED"
        assert client.command("INCRBY", "account:bob", 3) == b"QUEUED"
        assert client.command("EXEC") == [7, 3]
        assert client.command("MGET", "account:alice", "account:bob") == [b"7", b"3"]
    print("Transaction example passed")


if __name__ == "__main__":
    main()
