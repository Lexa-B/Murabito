"""The client against a stand-in bridge: a thread speaking the same frames."""

from __future__ import annotations

import socket
import threading

import pytest

from midbrain import murabito_pb2 as pb
from midbrain.client import LENGTH, Bridge, frame, read_frame


def test_a_frame_is_its_length_then_its_bytes() -> None:
    assert frame(b"hello") == b"\x00\x00\x00\x05hello"
    assert LENGTH.size == 4


class FakeBridge:
    """Answers one client the way the game does: snapshots on request, orders kept."""

    def __init__(self, bodies: list[pb.Snapshot]) -> None:
        self.bodies = bodies
        self.orders: list[pb.Order] = []
        self.listener = socket.create_server(("127.0.0.1", 0))
        self.port = self.listener.getsockname()[1]
        self.thread = threading.Thread(target=self.serve, daemon=True)
        self.thread.start()

    def serve(self) -> None:
        conn, _ = self.listener.accept()
        with conn:
            while True:
                try:
                    request = pb.Request()
                    request.ParseFromString(read_frame(conn))
                except ConnectionError:
                    return
                match request.WhichOneof("kind"):
                    case "snapshots":
                        conn.sendall(frame(pb.Snapshots(bodies=self.bodies).SerializeToString()))
                    case "order":
                        self.orders.append(request.order)


@pytest.fixture
def fake() -> FakeBridge:
    fox = pb.Snapshot(
        id=2,
        kind="murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox",
        tick=40,
        position=pb.Voxel(q=-8, r=0, layer=0),
        facing=pb.Direction.ESE,
        previous_outcome=pb.Outcome(idle=pb.Idle()),
    )
    return FakeBridge([fox])


def test_snapshots_come_back_as_the_bridge_sent_them(fake: FakeBridge) -> None:
    with Bridge(port=fake.port) as bridge:
        bodies = bridge.snapshots()
    assert len(bodies) == 1
    assert bodies[0].id == 2
    assert bodies[0].tick == 40
    assert pb.Direction.Name(bodies[0].facing) == "ESE"
    assert bodies[0].previous_outcome.WhichOneof("kind") == "idle"


def test_an_order_is_framed_and_names_the_body(fake: FakeBridge) -> None:
    with Bridge(port=fake.port) as bridge:
        bridge.order(2, pb.Intent(short=pb.Short(face=pb.Direction.N)))
        bridge.snapshots()  # a round trip, so the order has surely arrived
    assert len(fake.orders) == 1
    assert fake.orders[0].id == 2
    assert fake.orders[0].intent.short.WhichOneof("kind") == "face"
    assert fake.orders[0].intent.short.face == pb.Direction.N


def test_asking_before_connecting_is_an_error() -> None:
    with pytest.raises(ConnectionError):
        Bridge().snapshots()
