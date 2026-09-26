"""A client of the game's bridge: the mind's end of the port.

The bridge listens on the loopback address, port 15703, and speaks the contract in
``crates/ai/bridge/proto/murabito.proto``: a ``Request`` in, a ``Snapshots`` out, each
framed as a four-byte big-endian length and then the bytes. This module is the whole of
that on the Python side. Everything above it works with messages, never with sockets.
"""

from __future__ import annotations

import socket
import struct
from dataclasses import dataclass, field

from midbrain import murabito_pb2 as pb

HOST = "127.0.0.1"
PORT = 15703

LENGTH = struct.Struct(">I")
"""A frame's length prefix: four bytes, big-endian, as the bridge reads it."""


def frame(payload: bytes) -> bytes:
    """One frame: the length, then the bytes."""
    return LENGTH.pack(len(payload)) + payload


def read_exactly(sock: socket.socket, count: int) -> bytes:
    """That many bytes off the socket, or ``ConnectionError`` if it closes first."""
    chunks = bytearray()
    while len(chunks) < count:
        chunk = sock.recv(count - len(chunks))
        if not chunk:
            raise ConnectionError("the bridge hung up")
        chunks.extend(chunk)
    return bytes(chunks)


def read_frame(sock: socket.socket) -> bytes:
    """One frame off the socket, its length prefix stripped."""
    (length,) = LENGTH.unpack(read_exactly(sock, LENGTH.size))
    return read_exactly(sock, length)


@dataclass
class Bridge:
    """A connection to the game. Ask for snapshots, send orders."""

    host: str = HOST
    port: int = PORT
    timeout: float = 2.0
    _sock: socket.socket | None = field(default=None, repr=False)

    def connect(self) -> Bridge:
        self._sock = socket.create_connection((self.host, self.port), timeout=self.timeout)
        return self

    def close(self) -> None:
        if self._sock is not None:
            self._sock.close()
            self._sock = None

    def __enter__(self) -> Bridge:
        return self.connect()

    def __exit__(self, *_exc: object) -> None:
        self.close()

    @property
    def sock(self) -> socket.socket:
        if self._sock is None:
            raise ConnectionError("not connected: call connect() first")
        return self._sock

    def send(self, request: pb.Request) -> None:
        self.sock.sendall(frame(request.SerializeToString()))

    def snapshots(self) -> list[pb.Snapshot]:
        """Every body's latest snapshot, as of the game's last tick."""
        self.send(pb.Request(snapshots=pb.SnapshotsRequest()))
        answer = pb.Snapshots()
        answer.ParseFromString(read_frame(self.sock))
        return list(answer.bodies)

    def order(self, thing: int, intent: pb.Intent) -> None:
        """Tell a body, by its number, what to want. No reply: the next snapshot's
        ``previous`` says what became of it."""
        self.send(pb.Request(order=pb.Order(id=thing, intent=intent)))
