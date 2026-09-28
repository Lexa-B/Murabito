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
from midbrain.behaviour import Node, Result
from midbrain.beliefs import Belief, BelievedWorld, Cell
from midbrain.hexes import along, angle_of, apart, bearing, from_world, nearest_direction, opposite, rotated, steps, to_world, turn_between
from midbrain.paths import cells_along, landing, neighbour, plan
from midbrain.trees import Leaves, load

STOP = pb.Intent(short=pb.Short(stop=pb.Stop()))
TICKS_PER_SECOND = 64

PACES = ("walk", "jog", "sprint", "sneak")
"""The words a path may step in, as the contract spells them."""


def path(word: str, steps: list[int], keep: int = 0) -> pb.Intent:
    """A path of steps all in one pace word, or a turn each if the word is ``face``."""
    return pb.Intent(path=pb.Path(keep=keep, steps=[pb.Action(**{word: step}) for step in steps]))


def landing_of(cell: Cell, action: pb.Action) -> Cell:
    """Where an action leaves the body that starts it here: a step's neighbour, a lunge
    two cells on, a turn or a bite where it stands."""
    match action.WhichOneof("kind"):
        case "lunge":
            return neighbour(neighbour(cell, action.lunge), action.lunge)
        case "face" | "bite" | None:
            return cell
        case word:
            return neighbour(cell, getattr(action, word))


def steps_of(intent: pb.Intent) -> list[int] | None:
    """The directions a path intent steps in; None for anything else."""
    if intent.WhichOneof("kind") != "path":
        return None
    return [getattr(action, action.WhichOneof("kind")) for action in intent.path.steps]


def route_of(intent: pb.Intent | None, snapshot: pb.Snapshot) -> list[Cell]:
    """The cells a wanted path walks, from where the body will be when it starts; empty
    for anything but a path."""
    if intent is None:
        return []
    steps = steps_of(intent)
    return cells_along(origin_of(snapshot), steps) if steps is not None else []


def bound_for(intent: pb.Intent | None, snapshot: pb.Snapshot) -> Cell | None:
    """The cell a wanted path ends on; None for anything else, or a path of no steps."""
    route = route_of(intent, snapshot)
    return route[-1] if route else None


def origin_of(snapshot: pb.Snapshot) -> Cell:
    """Where the body will be when the next queued action starts: where the action
    underway lands, or where it stands."""
    here = Cell.of(snapshot.position)
    return landing_of(here, snapshot.underway) if snapshot.HasField("underway") else here


def face(direction: int) -> pb.Intent:
    return pb.Intent(short=pb.Short(face=direction))


def lunge(direction: int) -> pb.Intent:
    return pb.Intent(short=pb.Short(lunge=direction))


def sidestep(direction: int) -> pb.Intent:
    return pb.Intent(short=pb.Short(sidestep=direction))


EDGES = (0, 2, 4, 6, 8, 10)
"""The six edge directions: a step along one crosses a face, and a bite reaches only
across a face."""

LATERAL = (2, 3, 4, 8, 9, 10)
"""Notches off the facing that a sidestep may go: two to four either side."""


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

    @property
    def origin(self) -> Cell:
        """Where a path wanted this round starts: where the step underway lands."""
        return origin_of(self.snapshot)

    @property
    def blocked(self) -> set[Cell]:
        """Every cell the body believes something stands in: what a path walks round."""
        return {belief.cell for belief in self.world}

    def pace_to(self, word: str, cell: Cell) -> pb.Intent | None:
        """A path at that pace from the origin to the cell, planned round what is
        believed to stand in the way; None if no way is found, or something is believed
        to stand on the cell itself."""
        steps = plan(self.origin, cell, self.blocked)
        return path(word, steps) if steps is not None else None

    def walk_to(self, cell: Cell) -> pb.Intent | None:
        return self.pace_to("walk", cell)

    def jog_to(self, cell: Cell) -> pb.Intent | None:
        return self.pace_to("jog", cell)

    def sprint_to(self, cell: Cell) -> pb.Intent | None:
        return self.pace_to("sprint", cell)

    def sneak_to(self, cell: Cell) -> pb.Intent | None:
        return self.pace_to("sneak", cell)


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

    A bite reaches only across a face, so the line the fox pounces along, the **pounce
    line**, is always an edge direction: the thing's rear line when it faces an edge
    direction, else the edge direction 30° either side of dead behind that is nearer
    the fox, so it circles less and the choice holds once it is on it. A lunge covers
    two cells along the facing and the bite takes the third.

    The tree is ``trees/stalk.xml``, loaded with the leaves below; first branch to
    succeed wins:

        stalk
        ├─ freeze     it is looking at us              → Stop
        ├─ bite       the cell we face, across an      → Bite
        │             edge, is its
        ├─ pounce     facing along an edge, and the    → Lunge that way
        │             third cell that way is its
        ├─ set up     one sidestep would put us on     → Sidestep there
        │             the pounce line, facing it
        ├─ check      we have walked ``check_after``   → face where we believe it is; once
        │             cells without seeing it            facing, hold a round and count afresh
        │                                                 (revise forgets it if it isn't there
        │                                                 and is within reach; else we go on,
        │                                                 and closer)
        ├─ circle     we are off the pounce line       → SneakTo a cell one notch round toward
        │                                                 it, spiralling in by ``spiral``
        ├─ approach   on the pounce line, farther     → SneakTo the cell ``distance`` along it
        │             than ``distance``
        └─ watch                                       → face it, or hold if we already do

    "Looking at us" is within ``looking_arc`` centred on the way it was last seen facing;
    "on the pounce line" is within ``line_tolerance`` of it; "facing it" is the
    notch nearest the bearing to it, and a lunge goes only forward, so from ``distance``
    along the line the pounce fires once the watch has turned us. ``spiral`` is the
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
    line_tolerance: float = 15.0
    """Degrees either side of the pounce line that count as on it: half a notch, so the
    fox keeps circling until it is on the notch."""
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
        """The cell we face is its, across an edge: where a bite reaches."""
        target = self.target(ctx)
        return target is not None and ctx.facing in EDGES and neighbour(ctx.here, ctx.facing) == target.cell

    def a_lunge_lands_beside_it(self, ctx: Context) -> bool:
        """Facing along an edge direction with its cell the third that way: a lunge takes
        two and lands us facing it across the face."""
        target = self.target(ctx)
        return target is not None and ctx.facing in EDGES and along(ctx.here, ctx.facing, 3) == target.cell

    def pounce(self, ctx: Context) -> pb.Intent | None:
        return lunge(ctx.facing)

    def sidestep_onto_the_line(self, ctx: Context) -> int | None:
        """The lateral direction one sidestep of which would put a lunge beside it, if
        there is one: the pounce line has shifted a cell and we need not turn."""
        target = self.target(ctx)
        if target is None or ctx.facing not in EDGES:
            return None
        for notches in LATERAL:
            way = (ctx.facing + notches) % 12
            if along(neighbour(ctx.here, way), ctx.facing, 3) == target.cell:
                return way
        return None

    def a_sidestep_lines_up_a_lunge(self, ctx: Context) -> bool:
        return self.sidestep_onto_the_line(ctx) is not None

    def set_up(self, ctx: Context) -> pb.Intent | None:
        return sidestep(self.sidestep_onto_the_line(ctx))

    def pounce_line(self, ctx: Context, target: Belief) -> int:
        """The edge direction from it along which we line up: its rear if that is an edge
        direction, else whichever edge direction beside its rear is nearer our bearing
        from it; with its facing never seen, the edge direction nearest our bearing."""
        toward_us = bearing(target.cell, ctx.here)
        ideal = angle_of(opposite(target.facing)) if target.facing is not None else toward_us
        if ideal is None:
            return 0
        tie_break = toward_us if toward_us is not None else ideal
        return min(EDGES, key=lambda edge: (round(apart(angle_of(edge), ideal), 6), apart(angle_of(edge), tie_break)))

    def off_the_line(self, ctx: Context) -> bool:
        target = self.target(ctx)
        if target is None:
            return False
        toward_us = bearing(target.cell, ctx.here)
        return toward_us is not None and apart(angle_of(self.pounce_line(ctx, target)), toward_us) > self.line_tolerance

    def round_toward_the_line(self, ctx: Context) -> pb.Intent | None:
        target = self.target(ctx)
        toward_us = bearing(target.cell, ctx.here)
        line = angle_of(self.pounce_line(ctx, target))
        swing = 30.0 if turn_between(toward_us, line) > 0 else -30.0
        cell = rotated(ctx.here, target.cell, swing, pitch=self.spiral, floor=self.distance)
        return ctx.sneak_to(cell) if cell != ctx.here else None

    def settle_cell(self, ctx: Context, target: Belief) -> Cell:
        """The cell ``distance`` offsets from it along the pounce line."""
        return along(target.cell, self.pounce_line(ctx, target), self.distance)

    def farther_than_distance(self, ctx: Context) -> bool:
        """Farther from it than the settle cell is: in steps, since along a corner
        direction ``distance`` offsets is twice as many steps, and a fox standing on its
        settle cell must not be told it is still too far."""
        target = self.target(ctx)
        if target is None:
            return False
        return steps(ctx.here, target.cell) > steps(self.settle_cell(ctx, target), target.cell)

    def close_in(self, ctx: Context) -> pb.Intent | None:
        return ctx.sneak_to(self.settle_cell(ctx, self.target(ctx)))

    def watch(self, ctx: Context) -> pb.Intent | None:
        target = self.target(ctx)
        toward_it = bearing(ctx.here, target.cell)
        if toward_it is None:
            return None
        if nearest_direction(toward_it) == ctx.facing:
            return None
        return face_thing(target.id) if target.id in ctx.seen_now else face(nearest_direction(toward_it))

    @property
    def leaves(self) -> Leaves:
        """What the tree file may refer to, by ID."""
        return Leaves(
            conditions={
                "looking_at_us": self.looking_at_us,
                "beside_it_facing_it": self.beside_it_facing_it,
                "a_lunge_lands_beside_it": self.a_lunge_lands_beside_it,
                "a_sidestep_lines_up_a_lunge": self.a_sidestep_lines_up_a_lunge,
                "unseen_too_long": self.unseen_too_long,
                "off_the_pounce_line": self.off_the_line,
                "farther_than_distance": self.farther_than_distance,
            },
            acts={
                "stop": lambda ctx: STOP,
                "bite": lambda ctx: BITE,
                "lunge": self.pounce,
                "sidestep": self.set_up,
                "look": self.look_at_it,
                "round": self.round_toward_the_line,
                "close_in": self.close_in,
                "watch": self.watch,
            },
        )

    @property
    def tree(self) -> Node:
        return load(self.name, self.leaves)

    def want(self, ctx: Context) -> Result:
        if self.target(ctx) is None:
            return Result(False, None, (self.name, "no prey"))
        return self.tree.tick(ctx)


@dataclass(frozen=True)
class Wander:
    """Walk a leg somewhere ahead, rest a while, walk another. For a body with nowhere
    to be.

    The tree is ``trees/wander.xml``, loaded with the leaves below; first branch to
    succeed wins:

        wander
        ├─ walking   a path is in hand                → hold, let it land
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

    def a_path_in_hand(self, ctx: Context) -> bool:
        return ctx.in_hand is not None and ctx.in_hand.WhichOneof("kind") == "path"

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
        return ctx.walk_to(cell) if cell != ctx.here else None

    @property
    def leaves(self) -> Leaves:
        """What the tree file may refer to, by ID."""
        return Leaves(
            conditions={
                "a_path_in_hand": self.a_path_in_hand,
                "rest_not_over": self.rest_not_over,
                "no_rest_drawn": self.no_rest_drawn,
            },
            acts={
                "keep_on": lambda ctx: None,
                "wait": lambda ctx: None,
                "rest": self.draw_a_rest,
                "set_off": self.set_off,
            },
        )

    @property
    def tree(self) -> Node:
        return load(self.name, self.leaves)

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


def to_send(want: pb.Intent | None, snapshot: pb.Snapshot) -> pb.Intent | None:
    """What to send for a want, given what the body is doing: the want, an amendment of
    the path in hand, or nothing.

    A new short cuts whatever is in flight, so a mind must not re-send what is in hand.
    Nothing wanted is never sent. A Stop is sent only if something is in hand. A path
    wanted while a path is in hand is compared with what still waits on the queue: the
    steps that match from the front are kept, and only the rest is sent, as an
    amendment, so a target on the move re-plans the tail of the walk and never cuts the
    step in flight; a path that matches the queue whole is not sent. Anything else is
    sent when it isn't exactly what is in hand.
    """
    if want is None:
        return None
    doing = snapshot.doing.intent if snapshot.HasField("doing") else None
    if want == STOP:
        return STOP if doing is not None else None
    if doing is None:
        return want
    if want.WhichOneof("kind") == "path" and doing.WhichOneof("kind") == "path":
        wanted, queued = list(want.path.steps), list(snapshot.queue)
        keep = 0
        while keep < len(wanted) and keep < len(queued) and wanted[keep] == queued[keep]:
            keep += 1
        if keep == len(wanted) == len(queued):
            return None
        return pb.Intent(path=pb.Path(keep=keep, steps=wanted[keep:]))
    return want if want != doing else None
