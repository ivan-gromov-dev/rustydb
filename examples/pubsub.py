"""Deliver one binary-safe message to a subscribed client."""

import socket

from _client import Client, demo_server


def main():
    with demo_server() as publisher:
        connection = socket.create_connection(publisher.connection.getpeername(), timeout=5)
        subscriber = Client(connection)
        try:
            assert subscriber.command("SUBSCRIBE", "events") == [b"subscribe", b"events", 1]
            assert publisher.command("PUBLISH", "events", b"ready\x00now") == 1
            assert subscriber.read() == [b"message", b"events", b"ready\x00now"]
            assert subscriber.command("UNSUBSCRIBE", "events") == [b"unsubscribe", b"events", 0]
        finally:
            subscriber.reader.close()
            connection.close()
    print("Pub/Sub example passed")


if __name__ == "__main__":
    main()
