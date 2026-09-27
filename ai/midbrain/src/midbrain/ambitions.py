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

import math
import random
from dataclasses import dataclass, field
from typing import Protocol

from midbrain import murabito_pb2 as pb
from midbrain.behaviour import Act, Condition, Result, Selector, Sequence
from midbrain.beliefs import Belief, BelievedWorld, Cell
from midbrain.hexes import along, angle_of, apart, bearing, from_world, nearest_direction, opposite, rotated, steps, to_world, turn_between

STOP = pb.Intent(short=pb.Short(stop=pb.Stop()))
TICKS_PER_SECOND = 64


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


def lunge(direction: int) -> pb.Intent:
    return pb.Intent(short=pb.Short(lunge=direction))


BITE = pb.Intent(short=pb.Short(bite=pb.Bite()))


def face_thing(thing: int) -> pb.Intent:
    return pb.Intent(short=pb.Short(face_thing=thing))


def is_under(kind: str | None, ancestor: str) -> bool:
    """Whether a kind path is the ancestor or below it."""
    return kind is not None and (kind == ancestor or kind.startswith(ancestor + "::"))


@dataclass(frozen=True)
class Context:
    """What an ambition reads: one body's believed world and its snapshot this round, and
    the body's scratch, where an ambition keeps what it needs to remember between rounds
    that the board cannot show it (a rest that ends at a tick). Each ambition keeps to keys
    of its own name; the scratch is the body's alone, shared with no other body."""

    world: BelievedWorld
    snapshot: pb.Snapshot
    scratch: dict[str, object] = field(default_factory=dict)

    @property
    def here(self) -> Cell:
        return Cell.of(self.snapshot.position)

    @property
    def tick(self) -> int:
        return self.snapshot.tick

    @property
    def in_hand(self) -> pb.Intent | None:
        return self.snapshot.doing.intent if self.snapshot.HasField("doing") else None

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
    """Get behind the nearest believed thing of a kind, pounce, and bite.

    The tree, first branch to succeed wins:

        stalk
        ├─ freeze     it is looking at us              → Stop
        ├─ bite       beside it, facing it             → Bite
        ├─ pounce     behind it, facing it, and a      → Lunge that way
        │             lunge lands beside it
        ├─ check      we have walked ``check_after``   → face where we believe it is; once
        │             cells without seeing it            facing, hold a round and count afresh
        │                                                 (revise forgets it if it isn't there
        │                                                 and is within reach; else we go on,
        │                                                 and closer)
        ├─ circle     we are off its rear line         → SneakTo a cell one notch round toward
        │                                                 its rear, spiralling in by ``spiral``
        ├─ approach   on the rear line, farther than   → SneakTo the cell ``distance`` behind it
        │             ``distance``
        └─ watch                                       → face it, or hold if we already do

    "Looking at us" is within ``looking_arc`` centred on the way it was last seen facing;
    "on the rear line" is within ``rear_tolerance`` of dead behind it; "facing it" is the
    notch nearest the bearing to it, and a lunge goes only forward, so from ``distance``
    behind the pounce fires once the watch has turned us. ``spiral`` is the
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
        """Turn to face where it is believed; once facing it and still not seeing it, the
        check is done: the count starts again, so next round the stalk moves on (and closer,
        if it is beyond the reach within which ``revise`` would have forgotten it)."""
        target = self.target(ctx)
        toward_it = bearing(ctx.here, target.cell)
        if toward_it is None or nearest_direction(toward_it) == ctx.facing:
            target.walked_since = 0
            return None
        return face(nearest_direction(toward_it))

    def beside_it_facing_it(self, ctx: Context) -> bool:
        target = self.target(ctx)
        if target is None or steps(ctx.here, target.cell) != 1:
            return False
        return nearest_direction(bearing(ctx.here, target.cell)) == ctx.facing

    def a_lunge_lands_beside_it(self, ctx: Context) -> bool:
        target = self.target(ctx)
        if target is None or self.off_rear_line(ctx):
            return False
        toward_it = bearing(ctx.here, target.cell)
        if toward_it is None or nearest_direction(toward_it) != ctx.facing:
            return False
        return steps(along(ctx.here, ctx.facing, 2), target.cell) == 1

    def pounce(self, ctx: Context) -> pb.Intent | None:
        return lunge(ctx.facing)

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

    def settle_cell(self, ctx: Context, target: Belief) -> Cell:
        """The cell ``distance`` offsets behind it along its rear line, or, if its facing
        was never seen, ``distance`` offsets from it on the notch toward us."""
        if target.facing is not None:
            return along(target.cell, opposite(target.facing), self.distance)
        return along(target.cell, nearest_direction(bearing(target.cell, ctx.here)), self.distance)

    def farther_than_distance(self, ctx: Context) -> bool:
        """Farther from it than the settle cell is: in steps, since along a corner
        direction ``distance`` offsets is twice as many steps, and a fox standing on its
        settle cell must not be told it is still too far."""
        target = self.target(ctx)
        if target is None:
            return False
        return steps(ctx.here, target.cell) > steps(self.settle_cell(ctx, target), target.cell)

    def close_in(self, ctx: Context) -> pb.Intent | None:
        return sneak_to(self.settle_cell(ctx, self.target(ctx)))

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
            Sequence("bite", (Condition("beside it, facing it", self.beside_it_facing_it), Act("bite", lambda ctx: BITE))),
            Sequence("pounce", (Condition("a lunge lands beside it", self.a_lunge_lands_beside_it), Act("lunge", self.pounce))),
            Sequence("check", (Condition("unseen too long", self.unseen_too_long), Act("look", self.look_at_it))),
            Sequence("circle", (Condition("off its rear line", self.off_rear_line), Act("round", self.round_toward_rear))),
            Sequence("approach", (Condition("farther than distance", self.farther_than_distance), Act("close in", self.close_in))),
            Act("watch", self.watch),
        ))

    def want(self, ctx: Context) -> Result:
        if self.target(ctx) is None:
            return Result(False, None, (self.name, "no prey"))
        return self.tree.tick(ctx)


@dataclass(frozen=True)
class Wander:
    """Walk a leg somewhere ahead, rest a while, walk another. For a body with nowhere
    to be.

    The tree, first branch to succeed wins:

        wander
        ├─ walking   a pace is in hand                → hold, let it land
        ├─ resting   the rest drawn has not ended     → hold
        ├─ arrived   nothing in hand, no rest drawn   → draw a rest of ``rest`` seconds,
        │                                                remember when it ends; hold
        └─ set off   the rest is over                 → forget it; WalkTo a cell ``leg``
                                                         shaku away, within ``arc`` of the
                                                         way we face

    The leg's bearing and length are drawn from ``rng``; a test hands in a seeded one. The
    rest's end is the one thing the board cannot show, so it lives in the body's scratch
    under ``wander.rest_until``. On a body's first round it rests before its first leg.
    """

    leg: tuple[float, float] = (4.0, 12.0)
    """Shortest and longest leg, in shaku."""
    rest: tuple[float, float] = (2.0, 8.0)
    """Shortest and longest rest between legs, in seconds."""
    arc: float = 180.0
    """Degrees centred on the way we face within which a leg's bearing is drawn."""
    score: float = 0.3
    """What it bids: always a little, above idling, below anything with a reason."""
    rng: random.Random = field(default_factory=random.Random, compare=False)
    name: str = "wander"

    REST_UNTIL = "wander.rest_until"

    def utility(self, ctx: Context) -> float:
        return self.score

    # --- the tree's tests and wants ---

    def a_pace_in_hand(self, ctx: Context) -> bool:
        return ctx.in_hand is not None and bound_for(ctx.in_hand) is not None

    def rest_not_over(self, ctx: Context) -> bool:
        until = ctx.scratch.get(self.REST_UNTIL)
        return until is not None and ctx.tick < until

    def no_rest_drawn(self, ctx: Context) -> bool:
        return self.REST_UNTIL not in ctx.scratch

    def draw_a_rest(self, ctx: Context) -> pb.Intent | None:
        seconds = self.rng.uniform(*self.rest)
        ctx.scratch[self.REST_UNTIL] = ctx.tick + round(seconds * TICKS_PER_SECOND)
        return None

    def set_off(self, ctx: Context) -> pb.Intent | None:
        ctx.scratch.pop(self.REST_UNTIL, None)
        degrees = angle_of(ctx.facing) + self.rng.uniform(-self.arc / 2, self.arc / 2)
        length = self.rng.uniform(*self.leg)
        x, z = to_world(ctx.here)
        theta = math.radians(degrees)
        cell = from_world(x + length * math.cos(theta), z - length * math.sin(theta), ctx.here.layer)
        return walk_to(cell) if cell != ctx.here else None

    @property
    def tree(self) -> Selector:
        return Selector(self.name, (
            Sequence("walking", (Condition("a pace in hand", self.a_pace_in_hand), Act("keep on", lambda ctx: None))),
            Sequence("resting", (Condition("rest not over", self.rest_not_over), Act("wait", lambda ctx: None))),
            Sequence("arrived", (Condition("no rest drawn", self.no_rest_drawn), Act("rest", self.draw_a_rest))),
            Act("set off", self.set_off),
        ))

    def want(self, ctx: Context) -> Result:
        return self.tree.tick(ctx)


FOX = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"
HARE = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::hare"

REPERTOIRE: dict[str, list[Ambition]] = {
    FOX: [Idle(), Wander(), Stalk(prey=HARE)],
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
