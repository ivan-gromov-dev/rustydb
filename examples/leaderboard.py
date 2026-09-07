"""Update player scores, read a top list, and iterate the leaderboard."""
from _client import demo_server


def main():
    with demo_server() as client:
        assert client.command("ZADD", "board", 10, "alice", 15, "bob", 12, "carol") == 3
        assert client.command("ZINCRBY", "board", 6, "alice") == b"16"
        top = client.command("ZRANGE", "board", 0, 1, "REV", "WITHSCORES")
        assert top == [b"alice", b"16", b"bob", b"15"]
        assert client.command("ZREVRANK", "board", "alice") == 0
        assert client.command("ZMSCORE", "board", "carol", "missing") == [b"12", None]
        print("Top two:", top)
        cursor, players = b"0", []
        while True:
            cursor, batch = client.command("ZSCAN", "board", cursor, "COUNT", 2)
            players.extend(batch)
            if cursor == b"0":
                break
        assert players == [b"alice", b"16", b"bob", b"15", b"carol", b"12"]
        print("All players in member order:", players)
    print("Leaderboard example passed")


if __name__ == "__main__":
    main()
