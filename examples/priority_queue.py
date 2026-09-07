"""Consume lowest-priority-number jobs atomically with ZPOPMIN.

Members are unique job IDs. Equal priorities use binary job-ID ordering, not FIFO.
This is a priority queue, not an atomic delayed-job claim based on wall-clock time.
"""
from _client import demo_server


def main():
    with demo_server() as client:
        assert client.command("ZADD", "jobs", 20, "report", 1, "alert", 5, "email") == 3
        ready = client.command("ZRANGE", "jobs", "-inf", 5, "BYSCORE")
        assert ready == [b"alert", b"email"]
        print("Jobs with priority at most five:", ready)
        first = client.command("ZPOPMIN", "jobs")
        assert first == [b"alert", b"1"]
        print("Claimed:", first)
        remaining = client.command("ZPOPMIN", "jobs", 10)
        assert remaining == [b"email", b"5", b"report", b"20"]
        assert client.command("ZPOPMIN", "jobs") == []
        assert client.command("TYPE", "jobs") == b"none"
        print("Claimed remaining jobs:", remaining)
    print("Priority queue example passed")


if __name__ == "__main__":
    main()
