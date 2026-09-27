"""What a body believes about the world: the things it has seen, frozen where it last saw them.

The first, smallest believed world. Each body on the board has one. Every round the mind
feeds it the body's snapshot, and every sighting in it becomes a belief, or refreshes one:
what the thing is, which cell it was in, when that was. Nothing is ever forgotten, nothing
moves on its own, nothing fades. Those come later, here, without touching the wire.

A belief's cell is absolute. The wire says "four steps south of me"; the belief says "at
(-8, 4)", so it still means something once the body has walked away.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from midbrain import murabito_pb2 as pb


@dataclass(frozen=True)
class Cell:
    """One cell of the hex plane, axial, and its layer; the mind's own copy of a voxel."""

    q: int
    r: int
    layer: int = 0

    @property
    def s(self) -> int:
        return -self.q - self.r

    @classmethod
    def of(cls, voxel: pb.Voxel) -> Cell:
        return cls(voxel.q, voxel.r, voxel.layer)

    def plus(self, offset: pb.Offset) -> Cell:
        """The cell this offset away."""
        return Cell(self.q + offset.dq, self.r + offset.dr, self.layer + offset.dlayer)

    def __str__(self) -> str:
        return f"({self.q}, {self.r}, {self.s}) L{self.layer}"


@dataclass
class Belief:
    """One thing, as last seen."""

    id: int
    kind: str | None
    """Its path in the tree of kinds, or None if the sighting carried no label."""
    cell: Cell
    """Where it was when last seen."""
    seen_at: int
    """The tick of that sighting."""
    acuity: int
    """How well it was seen then: a ``pb.Acuity`` value."""
    facing: int | None
    """The way it faced then, a ``pb.Direction`` value, or None if it has no facing."""

    def age(self, now: int) -> int:
        """Ticks since the last sighting."""
        return now - self.seen_at


@dataclass
class BelievedWorld:
    """Everything one body believes, keyed on the believed thing's id."""

    body: int
    """The id of the body whose beliefs these are."""
    beliefs: dict[int, Belief] = field(default_factory=dict)

    def observe(self, snapshot: pb.Snapshot) -> None:
        """Takes in one round's snapshot: every sighting becomes, or refreshes, a belief."""
        if snapshot.id != self.body:
            raise ValueError(f"snapshot of #{snapshot.id} given to #{self.body}'s beliefs")
        here = Cell.of(snapshot.position)
        for sighting in snapshot.in_view:
            self.beliefs[sighting.id] = Belief(
                id=sighting.id,
                kind=sighting.kind if sighting.HasField("kind") else None,
                cell=here.plus(sighting.offset),
                seen_at=snapshot.tick,
                acuity=sighting.acuity,
                facing=sighting.facing if sighting.HasField("facing") else None,
            )

    def __iter__(self):
        """The beliefs, by the believed thing's id."""
        return iter(sorted(self.beliefs.values(), key=lambda belief: belief.id))

    def __len__(self) -> int:
        return len(self.beliefs)

    def get(self, thing: int) -> Belief | None:
        return self.beliefs.get(thing)
