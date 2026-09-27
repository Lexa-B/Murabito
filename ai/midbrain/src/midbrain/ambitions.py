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


def _to(pace: str, cell: Cell) -> pb.Intent:
    voxel = pb.Voxel(q=cell.q, r=cell.r, layer=cell.layer)
    return pb.Intent(sustained=pb.Sustained(**{pace: voxel}))


def walk_to(cell: Cell) -> pb.Intent:
    return _to("walk_to", cell)


def jog_to(cell: Cell) -> pb.Intent:
    return _to("jog_to", cell)


def sprint_to(cell: Cell) -> pb.Intent:
    return _to("sprint_to", cell)


def sneak_to(cell: Cell) -> pb.Intent:
    return _to("sneak_to", cell)


def bound_for(intent: pb.Intent) -> Cell | None:
    """The cell a sustained intent is bound for, whatever its pace; None for a short."""
    if intent.WhichOneof("kind") != "sustained":
        return None
    pace = intent.sustained.WhichOneof("kind")
    return Cell.of(getattr(intent.sustained, pace)) if pace else None


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
        ├─ check      we have walked ``check_after``   → face where we believe it is; hold
        │             cells without seeing it            once facing (revise then forgets it
        │                                                 if it isn't there)
        ├─ circle     we are off its rear line         → SneakTo a cell one notch round toward
        │                                                 its rear, spiralling in by ``spiral``
        ├─ approach   on the rear line, farther than   → SneakTo the cell ``distance`` behind it
        │             ``distance``
        └─ watch                                       → face it, or hold if we already do

    "Looking at us" is within ``looking_arc`` centred on the way it was last seen facing;
    "on the rear line" is within ``rear_tolerance`` of dead behind it. ``spiral`` is the
    circle's pitch: 0 keeps our distance, a pure arc; a positive angle tilts each swing that
    far inward, so we close in as we come round, never nearer than ``distance``. A thing whose facing
    was never seen is taken as not looking and approached straight.
    """

    prey: str
    """The kind path to stalk: the kind itself or anything under it."""
    distance: int = 3
    """How many cells behind it to settle."""
    looking_arc: float = 180.0
    """Degrees of its front within which it counts as looking at us."""
    rear_tolerance: float = 15.0
    """Degrees either side of dead behind that count as on its rear line: half a notch, so
    the fox keeps circling until it is on the notch dead behind."""
    spiral: float = 15.0
    """Degrees the circling path tilts inward, off the tangent; 0 is a pure arc."""
    check_after: int = 3
    """Cells to walk without a sighting before pivoting to check it is where we left it."""
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

    def unseen_too_long(self, ctx: Context) -> bool:
        target = self.target(ctx)
        return target is not None and target.id not in ctx.seen_now and target.walked_since >= self.check_after

    def look_at_it(self, ctx: Context) -> pb.Intent | None:
        toward_it = bearing(ctx.here, self.target(ctx).cell)
        if toward_it is None or nearest_direction(toward_it) == ctx.facing:
            return None
        return face(nearest_direction(toward_it))

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
        cell = rotated(ctx.here, target.cell, swing, pitch=self.spiral, floor=self.distance)
        return sneak_to(cell) if cell != ctx.here else None

    def farther_than_distance(self, ctx: Context) -> bool:
        target = self.target(ctx)
        return target is not None and steps(ctx.here, target.cell) > self.distance

    def close_in(self, ctx: Context) -> pb.Intent | None:
        target = self.target(ctx)
        if target.facing is not None:
            return sneak_to(along(target.cell, opposite(target.facing), self.distance))
        toward_us = bearing(target.cell, ctx.here)
        return sneak_to(along(target.cell, nearest_direction(toward_us), self.distance))

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
            Sequence("check", (Condition("unseen too long", self.unseen_too_long), Act("look", self.look_at_it))),
            Sequence("circle", (Condition("off its rear line", self.off_rear_line), Act("round", self.round_toward_rear))),
            Sequence("approach", (Condition("farther than distance", self.farther_than_distance), Act("close in", self.close_in))),
            Act("watch", self.watch),
        ))

    def want(self, ctx: Context) -> Result:
        if self.target(ctx) is None:
            return Result(False, None, (self.name, "no prey"))
        return self.tree.tick(ctx)


FOX = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"
HARE = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::hare"

REPERTOIRE: dict[str, list[Ambition]] = {
    FOX: [Idle(), Stalk(prey=HARE)],
}
"""What each kind can want, keyed on its path. A kind not listed only idles."""


def repertoire_for(kind: str) -> list[Ambition]:
    return REPERTOIRE.get(kind, [Idle()])


def revise(ctx: Context, reach: int = 18) -> list[int]:
    """Drops a belief the body's own eyes contradict, and returns what was dropped: a thing
    believed right here or next door that is not in view; and a thing believed within
    ``reach`` cells (the fox's near band, 18), on the notch the body is facing, that is not
    in view. Standing on it or looking straight at it and seeing nothing means it is gone."""

    def contradicted(belief: Belief) -> bool:
        if belief.id in ctx.seen_now:
            return False
        if steps(ctx.here, belief.cell) <= 1:
            return True
        toward_it = bearing(ctx.here, belief.cell)
        return steps(ctx.here, belief.cell) <= reach and nearest_direction(toward_it) == ctx.facing

    gone = [belief.id for belief in ctx.world if contradicted(belief)]
    for thing in gone:
        ctx.world.forget(thing)
    return gone


def choose(ambitions: list[Ambition], ctx: Context, current: str | None, boost: float = 0.15) -> Ambition:
    """The ambition that scores highest, the one in hand boosted so a near tie holds. An
    ambition bidding nothing at all gets no boost: what has lost its reason to run is let go."""

    def score(ambition: Ambition) -> float:
        utility = ambition.utility(ctx)
        return utility + (boost if utility > 0 and ambition.name == current else 0.0)

    return max(ambitions, key=score)


def differs(want: pb.Intent | None, snapshot: pb.Snapshot, slack: int = 2) -> bool:
    """Whether a want is worth sending, given what the body is doing.

    A new intent cuts short whatever is in flight, so a mind must not re-send what is in
    hand. Nothing wanted is never sent. A Stop is sent only if something is in hand. A
    sustained pace is sent only if nothing is in hand, or the one in hand is another pace,
    or its target is ``slack`` or more cells from the one wanted, so a target creeping a
    cell at a time doesn't cut every step. Anything else is sent when it isn't exactly what
    is in hand.
    """
    if want is None:
        return False
    doing = snapshot.doing.intent if snapshot.HasField("doing") else None
    if want == STOP:
        return doing is not None
    if doing is None:
        return True
    wanted, in_hand = bound_for(want), bound_for(doing)
    if wanted is not None and in_hand is not None:
        same_pace = want.sustained.WhichOneof("kind") == doing.sustained.WhichOneof("kind")
        return not same_pace or steps(wanted, in_hand) >= slack
    return want != doing
