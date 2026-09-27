"""Ambitions: what a body could be trying to do, each scored, the best one running a tree.

Halo's shape. Every round, each ambition in a body's repertoire reads the believed world
and the body's snapshot and says how much it wants to run, 0 to 1; ``choose`` takes the
highest, with a small boost for the one already running so a near tie doesn't flip it
every round. The chosen ambition then ticks its behaviour tree for the intent it wants.

The Sims' half of the design, things in the believed world advertising what they offer
and motives weighting them, is the hook these utilities will hang on later; for now a
utility is a plain function.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol

from midbrain import murabito_pb2 as pb
from midbrain.behaviour import Act, Condition, Result, Selector, Sequence
from midbrain.beliefs import Belief, BelievedWorld, Cell
from midbrain.hexes import along, angle_of, apart, bearing, nearest_direction, opposite, rotated, steps, turn_between

STOP = pb.Intent(short=pb.Short(stop=pb.Stop()))


def go_to(cell: Cell) -> pb.Intent:
    return pb.Intent(sustained=pb.Sustained(go_to=pb.Voxel(q=cell.q, r=cell.r, layer=cell.layer)))


def face(direction: int) -> pb.Intent:
    return pb.Intent(short=pb.Short(face=direction))


def face_thing(thing: int) -> pb.Intent:
    return pb.Intent(short=pb.Short(face_thing=thing))


def is_under(kind: str | None, ancestor: str) -> bool:
    """Whether a kind path is the ancestor or below it."""
    return kind is not None and (kind == ancestor or kind.startswith(ancestor + "::"))


@dataclass(frozen=True)
class Context:
    """What an ambition reads: one body's believed world and its snapshot this round."""

    world: BelievedWorld
    snapshot: pb.Snapshot

    @property
    def here(self) -> Cell:
        return Cell.of(self.snapshot.position)

    @property
    def facing(self) -> int:
        return self.snapshot.facing

    @property
    def seen_now(self) -> set[int]:
        return {sighting.id for sighting in self.snapshot.in_view}


class Ambition(Protocol):
    name: str

    def utility(self, ctx: Context) -> float: ...

    def want(self, ctx: Context) -> Result: ...


@dataclass(frozen=True)
class Idle:
    """Stand there. Always a little wanted, so a body with nothing better does nothing."""

    score: float = 0.1
    name: str = "idle"

    def utility(self, ctx: Context) -> float:
        return self.score

    def want(self, ctx: Context) -> Result:
        return Result(True, None, (self.name,))


@dataclass(frozen=True)
class Stalk:
    """Get behind the nearest believed thing of a kind and stay there, facing it.

    The tree, first branch to succeed wins:

        stalk
        ├─ freeze     it is looking at us              → Stop
        ├─ circle     we are off its rear line         → GoTo a cell at our distance, one
        │                                                 notch round toward its rear
        ├─ approach   on the rear line, farther than   → GoTo the cell ``distance`` behind it
        │             ``distance``
        └─ watch                                       → face it, or hold if we already do

    "Looking at us" is within ``looking_arc`` centred on the way it was last seen facing;
    "on the rear line" is within ``rear_tolerance`` of dead behind it. A thing whose facing
    was never seen is taken as not looking and approached straight.
    """

    prey: str
    """The kind path to stalk: the kind itself or anything under it."""
    distance: int = 3
    """How many cells behind it to settle."""
    looking_arc: float = 180.0
    """Degrees of its front within which it counts as looking at us."""
    rear_tolerance: float = 30.0
    """Degrees either side of dead behind that count as on its rear line."""
    name: str = "stalk"

    def target(self, ctx: Context) -> Belief | None:
        prey = [belief for belief in ctx.world if is_under(belief.kind, self.prey)]
        return min(prey, key=lambda belief: steps(ctx.here, belief.cell), default=None)

    def utility(self, ctx: Context) -> float:
        return 1.0 if self.target(ctx) is not None else 0.0

    # --- the tree's tests and wants; each takes the context and reads the target afresh ---

    def looking_at_us(self, ctx: Context) -> bool:
        target = self.target(ctx)
        if target is None or target.facing is None:
            return False
        toward_us = bearing(target.cell, ctx.here)
        return toward_us is not None and apart(angle_of(target.facing), toward_us) < self.looking_arc / 2

    def off_rear_line(self, ctx: Context) -> bool:
        target = self.target(ctx)
        if target is None or target.facing is None:
            return False
        toward_us = bearing(target.cell, ctx.here)
        return toward_us is not None and apart(angle_of(opposite(target.facing)), toward_us) > self.rear_tolerance

    def round_toward_rear(self, ctx: Context) -> pb.Intent | None:
        target = self.target(ctx)
        toward_us = bearing(target.cell, ctx.here)
        rear = angle_of(opposite(target.facing))
        swing = 30.0 if turn_between(toward_us, rear) > 0 else -30.0
        cell = rotated(ctx.here, target.cell, swing)
        return go_to(cell) if cell != ctx.here else None

    def farther_than_distance(self, ctx: Context) -> bool:
        target = self.target(ctx)
        return target is not None and steps(ctx.here, target.cell) > self.distance

    def close_in(self, ctx: Context) -> pb.Intent | None:
        target = self.target(ctx)
        if target.facing is not None:
            return go_to(along(target.cell, opposite(target.facing), self.distance))
        toward_us = bearing(target.cell, ctx.here)
        return go_to(along(target.cell, nearest_direction(toward_us), self.distance))

    def watch(self, ctx: Context) -> pb.Intent | None:
        target = self.target(ctx)
        toward_it = bearing(ctx.here, target.cell)
        if toward_it is None:
            return None
        if nearest_direction(toward_it) == ctx.facing:
            return None
        return face_thing(target.id) if target.id in ctx.seen_now else face(nearest_direction(toward_it))

    @property
    def tree(self) -> Selector:
        return Selector(self.name, (
            Sequence("freeze", (Condition("looking at us", self.looking_at_us), Act("stop", lambda ctx: STOP))),
            Sequence("circle", (Condition("off its rear line", self.off_rear_line), Act("round", self.round_toward_rear))),
            Sequence("approach", (Condition("farther than distance", self.farther_than_distance), Act("close in", self.close_in))),
            Act("watch", self.watch),
        ))

    def want(self, ctx: Context) -> Result:
        return self.tree.tick(ctx)


def revise(ctx: Context) -> list[int]:
    """Drops any belief in a thing right here, or next door, that is not in view: if I am
    standing where I believe it is and it isn't there, it is gone. Returns what was dropped."""
    gone = [b.id for b in ctx.world if steps(ctx.here, b.cell) <= 1 and b.id not in ctx.seen_now]
    for thing in gone:
        ctx.world.forget(thing)
    return gone


def choose(ambitions: list[Ambition], ctx: Context, current: str | None, boost: float = 0.15) -> Ambition:
    """The ambition that scores highest, the one in hand boosted so a near tie holds."""

    def score(ambition: Ambition) -> float:
        return ambition.utility(ctx) + (boost if ambition.name == current else 0.0)

    return max(ambitions, key=score)


def differs(want: pb.Intent | None, snapshot: pb.Snapshot, slack: int = 2) -> bool:
    """Whether a want is worth sending, given what the body is doing.

    A new intent cuts short whatever is in flight, so a mind must not re-send what is in
    hand. Nothing wanted is never sent. A Stop is sent only if something is in hand. A
    GoTo is sent only if nothing is in hand, or the target in hand is ``slack`` or more
    cells from the one wanted, so a target creeping a cell at a time doesn't cut every
    step. Anything else is sent when it isn't exactly what is in hand.
    """
    if want is None:
        return False
    doing = snapshot.doing.intent if snapshot.HasField("doing") else None
    if want == STOP:
        return doing is not None
    if doing is None:
        return True
    if want.WhichOneof("kind") == "sustained" and doing.WhichOneof("kind") == "sustained":
        wanted, in_hand = Cell.of(want.sustained.go_to), Cell.of(doing.sustained.go_to)
        return steps(wanted, in_hand) >= slack
    return want != doing
