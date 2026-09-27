# The AI's I/O

A brief on how a mind meets a body: the three crates under `crates/ai/` and the Python project
under `ai/midbrain/`, as agreed in design on 2026-09-26 and built through 2026-09-27, with the
mind's believed world begun the same day. Implemented as described unless marked as design.

## The shape of it

Three layers, and a seam between the lower two and the top one.

- **The brainstem** (`murabito_brainstem`) runs every tick, inside the game. It holds one
  intent per body, turns it into actions on the body's `ActionQueue`, and decides nothing. In
  the driver's seat, not in charge: with nothing to do, a body stands there, since a rest is an
  empty queue.
- **Reflexes** (`murabito_reflexes`) are what a body does without thinking: short, hard-wired
  reactions a kind has, with dials an individual may have turned. They run every tick too, and
  a firing reflex takes the body away from whatever the mind asked for.
- **The midbrain** (`ai/midbrain/`) is the slow mind. It runs outside the game process, on its
  own clock, about every 125 ms, in Python. It reads a snapshot of every body and tells each
  what to want. Its first behaviour is design (below); what exists is the wire, a live view of
  the board, an order given by hand, and the first **believed world**: what each body has seen,
  frozen where it last saw it, the layer between what the brainstem tells the mind and what the
  mind will decide on.

Between the brainstem and the midbrain sits **the port**: a board of snapshots going up, a
channel of orders coming down. Whatever holds the port's far ends is the mind. Inside the game
that is a headless test; outside it, **the bridge** (`murabito_bridge`) holds the ends and serves
them on a loopback socket in a protobuf contract.

Arrows point down to whoever owns a type: the midbrain speaks to the bridge over the wire,
the bridge and the reflexes depend on the brainstem, `murabito_kinds` depends on brainstem and
reflexes (a `Sentient` requires a `Brainstem` and a `Reflexes`), and the brainstem depends on
the senses and the queue below it. The brainstem never learns a fox, a reflex or a socket exists.

## The vocabulary: `Short` and `Sustained`

The brainstem owns the words a body can be told, in two tiers, and the compiler holds the line
between them.

```rust
pub enum Short {            // over within one round of the mind: a single motion, or none
    Stop,
    Face(Direction),
    FaceThing(ThingId),     // toward where the thing is seen now; Lost if it isn't in view
    Walk(Direction),        // the action words of actions_readme.md, one to one:
    Jog(Direction),         // the paces turn first if need be
    Sprint(Direction),
    Sneak(Direction),
    Sidestep(Direction),    // the way words are only that way of the body's facing,
    Backstep(Direction),    // and are Refused otherwise
    Recoil(Direction),
    Lunge(Direction),
    Bite,
}

pub enum Sustained {        // outlives rounds; drive re-aims it every step: a cell, at a pace
    WalkTo(VoxelCoord),     // on the plane; the layer is not walked
    JogTo(VoxelCoord),      // each step the pace's word: Jog, Sprint, Sneak
    SprintTo(VoxelCoord),
    SneakTo(VoxelCoord),
}

pub enum Intent { Short(Short), Sustained(Sustained) }
```

A reflex's code returns an `Option<Short>`, by type, so a reflex can never hold a body for
longer than one round; the rule is structural. A `Sustained` intent has no plan: at every
landing, drive picks the next step afresh from where the body stands and what it sees, so a
moved target or a superseded order needs no replanning. A pace to a cell picks the step that promises the
shortest walk, the step's own cost in shaku plus the straight line left from where it lands,
so a corner step (√3 shaku) is taken when it truly cuts the corner and never to zig-zag.

A `Short` that is an action word is that word and nothing under it: the brainstem never sees a
gait or a reach, and a mind cannot ask for one. It asks `Action::check` against the body's own
facing before pushing, and ends a wrong-way word as `Refused` with the reason, so the mind hears
through `previous` and not the log. New words are new variants: `Follow(ThingId)`, `Flee(ThingId)`
are the ones named so far, each a few lines in drive when a mind asks for it (design).

## The body's driver: `Brainstem`

Every sentient carries a `Brainstem`. It holds what is pending (given, not yet begun), what is
being done (`Doing`: the intent and the tick it began) and what was done last (`Previous`: the
intent, and how it ended). Two ways in, `order(intent)` for anyone and `preempt(short, name)`
for a reflex; two ways to read, `doing()` and `previous()`. Both writes take effect at the next
tick's drive; two in one tick and the later wins.

```rust
pub enum Outcome {
    Done,
    Stopped,                  // a Stop replaced it
    Superseded,               // a newer order replaced it
    Cancelled(&'static str),  // a reflex, by name, replaced it
    Lost(ThingId),            // the thing it named was not in view
    Refused(&'static str),    // the body could not be asked it the way it faced: "not lateral"
}
```

`previous` is `None` until something has been asked. It is the mind's only feedback: it learns
its order was dropped, by what, and what the body did in its place.

### The tick

Three steps inside `murabito_actions::AskingSet`, chained, in `BrainstemSet`:

1. **Orders.** The tick is counted and the port's channel is drained; each order goes to the
   body it names, and one for a body not in the world is dropped.
2. **Reflexes.** The slot the reflexes crate fills. After orders, so a fright beats an order from
   the same tick.
3. **Drive.** A new intent takes the body now: the queue is cleared and, if something is in
   flight, the body is marked `CutShort` (actions' mark: the `Step` or `Turn` is dropped and the
   bar abandoned before `issue` runs on this same tick). Then, for a `Short`, its one action is
   pushed once and the intent is done when the body is idle again (queue empty, bar not in
   flight); for a `Sustained`, the next step is pushed whenever the body is idle, and it is done
   when nothing is left.

Then the mechanisms move the body, perception rebuilds `Occupancy` and every eye looks, and
**publish** writes every body's `Snapshot` to the board, after `PerceptionSet::Sense`, so the
picture is this tick's.

Two consequences to know. A cut step never happened: a body's place changes only on landing, so
nothing snaps back on screen, the bar just vanishes; a cut turn keeps the notches already made,
and nothing carries. And the view is a tick old: `Sense` closes a tick and drive opens the next,
so a body told to face something before it has ever looked reports it `Lost`.

Traces: a reflex firing is logged at `info`; drive's aims and steps at `debug`
(`RUST_LOG=murabito_brainstem=debug`).

## Reflexes

A reflex is one trigger to one response, named `category_reaction_trigger`, with its dials as
its fields and its code in `Reflex::check`, a pure function of what the body sees now, the ids
it saw last tick, and a way to ask what kind a seen thing is. The catalogue so far:

- `startle_face_apparition { by: Kind }`: something of that kind, or under it, is in view at
  `Near` acuity this tick and was not in view at all last tick; the body turns to face it, the
  nearest if several appeared. The fox's says `Sentient::KIND`, so a tree, however suddenly
  seen, is scenery.

A body's `Reflexes` is its repertoire: `Wired` entries, a reflex and a priority. The kind gives
it in its own `require`, the way it gives its `Vision`; `Sentient` requires an empty one. An
individual may be spawned with a repertoire `tuned`, "a tad jumpy", since what is given at
spawn wins; that the repertoire stays the kind's and only the dials move is convention, not
enforced. Each tick, one system in the `Reflexes` slot runs every body's repertoire and
preempts the brainstem with the highest-priority match, the earlier entry on a tie.

Two ticks never fire, both found in the game's own log: a body whose eyes were only just added
has not looked yet, so its empty view is not remembered as a look, and nothing startles at the
world's first sight of it; and a tick on which the body faces a new way since its last look,
since it revealed whatever is new by turning. The rest of that judgement, newly *noticed*
against newly *known*, is a believed world (`TODO.md`).

## The port

One resource, `Port`, owned by the brainstem, with two halves that want opposite rules.

- **The board**, going up: one `Snapshot` per body, the whole board rewritten every tick after
  the senses, latest wins, read at any moment by whoever holds a `Board` handle. A body that
  leaves the world leaves the board.
- **Orders**, coming down: a channel of `Order { id: ThingId, intent }`, every one kept in
  order, drained once a tick. Anyone holding an `Orders` handle sends.

Both handles are plain `Send` values, so a socket thread or a headless test can be the mind; a
test in the brainstem drives a body from a second thread through them.

A snapshot is flat facts with names, built for a scorer to read:

| Field | What |
|---|---|
| `id`, `kind` | the body's number and its path in the tree of kinds |
| `tick` | which tick this is a picture of |
| `position`, `facing` | the voxel as in memory, `{q, r, layer}`, and the compass direction |
| `in_view` | per sighting: `id`, `kind` (or none if the thing carries no label), `offset`, `distance` in steps, `acuity`, `facing` (the way the seen thing faces, or none) |
| `doing` | the intent and the tick it began, or nothing |
| `queue` | every action waiting, first to last |
| `in_flight` | the fraction of the current step, or nothing |
| `previous` | what was done last and how it ended, or nothing yet |

The engine's `Entity` handle stays out of it: a mind keys on `ThingId`.

## The bridge and the contract

`murabito_bridge` holds the port's far ends and serves them on `127.0.0.1:15703`, always on,
one thread per client. Frames are a four-byte big-endian length then protobuf bytes, capped at
a megabyte. The contract is one file, `crates/ai/bridge/proto/murabito.proto`, package
`murabito.ai`: a `Request` in, either a `SnapshotsRequest` or an `Order`; a `Snapshots` out, in
answer to the former. An order gets no reply; what became of it is the body's next `previous`.
The shapes mirror the brainstem's own: a voxel is axial, an enum's number is its index in
memory, the vocabulary is nested `oneof`s so a client can ask `WhichOneof("kind")`.

The `.proto` compiles at build time with `protox`, a protobuf compiler that is itself a crate,
so no `protoc` need be installed; `prost-build` writes the Rust module. `wire.rs` converts at
the edge, outward without loss and inward refusing a malformed message with a word on what was
missing. Bytes that are no request, or an absurd length, close that one client; if the port is
taken at start, the game logs an error and runs without a bridge. Several clients may connect.

`ThingId::restored(number)` exists for this: an order names its body by number, and a number
nothing has names nothing.

## The midbrain's home: `ai/midbrain/`

A `uv` project on Python 3.13, the first client of the bridge.

```
uv run board                 # every body's snapshot, live, redrawn every 125 ms
uv run order 3 walkto 0 0    # one order by hand: stop | face DIR | face-thing N | walkto Q R [LAYER]
                             #   or jogto, sprintto, sneakto
uv run order 1 recoil W      # or walk, jog, sprint, sneak, sidestep, backstep, lunge DIR; bite
uv run mind                  # what every body believes, live, in the terminal
uv run mind --visualize      # one body's believed world drawn in a window; --body N, Tab cycles
uv run pytest
./regen.sh                   # after the .proto changes
```

`client.py` is the whole wire on this side: the frames, `snapshots()`, `order()`.
`murabito_pb2.py` is generated from the one contract with `grpcio-tools` and checked in, so
`uv run` needs no build step; a test regenerates it and fails if the contract moved on.
`order.py` turns words into an intent through one pure function that refuses what it can't read
with a word on what it wanted; the midbrain's first behaviour calls the same thing with its own
words. `board.py` draws one panel per body.

### The believed world

A mind does not act on what a body sees; it acts on what the body *believes*, and the two part
ways the moment the body looks away. `beliefs.py` is that layer, in its smallest form. Each body
on the board has a `BelievedWorld`, and in it a `Belief` per thing the body has ever seen, keyed
on the thing's id: its kind path (or none, if the sighting carried no label), the cell it stood
in, the way it faced, the tick it was last seen, the acuity then, and how many cells the body
has walked since (`walked_since`, the seed of uncertainty later). The cell is absolute, the body's own voxel
plus the sighting's offset, so "four steps south of me" is remembered as "at (-8, 4)" and still
means something once the body has walked on. One verb, `observe(snapshot)`, writes every
sighting in over what was there; a snapshot from another body is refused.

Nothing is forgotten, nothing moves on its own, nothing fades: a thing out of view stays where
it was last seen and only its age grows. That is the whole model for now, chosen so it could be
watched and validated before it grows (below). It runs in the mind, once a round, so a sighting
shorter than a round can slip between snapshots; that is the midbrain's nature.

`mind.py` is the loop: a `Mind` holds every body's world and the latest snapshot each gave (a
body's own place and facing are truth, not belief), and each round pulls the board and observes
every snapshot. It decides nothing and sends nothing yet. `visualize.py` draws one body's world
in a pygame window, flat and top-down in the game's own geometry (pointy-top cells, `x = q +
r/2`, `z = r·√3/2`, north up): the body at its true cell with a line for its facing, each
believed thing a filled hex where it was last seen, coloured by kind and labelled with what it
is and how long ago, outlined if in view this round. The game window beside it is the truth;
nothing of the truth is drawn here. Layers are tracked in the model and shown in the terminal
view, and flattened in the window until something stands on one.

### Ambitions, and the stalk

The mind decides the way Halo Infinite's bots do, with the Sims' half left as a hook. Every
round, each **ambition** in a body's repertoire (`ambitions.py`, `REPERTOIRE`, keyed on the
kind's path; a kind not listed only idles) reads the believed world and the snapshot and bids
a utility, 0 to 1; `choose` takes the highest, with a small boost for the one in hand so a
near tie doesn't flip every round. The winner then ticks its **behaviour tree**
(`behaviour.py`: `Condition`, `Act`, `Selector`, `Sequence`, ticked afresh from the root each
round, no memory of its own) for the intent it wants, and the loop sends that only if it
**differs** from what the body is doing: a hold is never sent, a `Stop` only when something is
in hand, a pace to a cell only when nothing is in hand, the pace in hand is another, or the
target has moved two or more cells, so the mind never cuts its own steps. Before bidding, `revise` drops a belief the body's own eyes
contradict: a thing that should be within a cell and isn't in view, or one on the notch the
body faces within its near reach (18 cells, the fox's near band, copied) and not in view.

The fox's repertoire is `Idle` and `Stalk(prey=hare, distance=3, looking_arc=180,
rear_tolerance=15, spiral=15, check_after=3)`, which bids 1 while it believes in a hare and
runs this tree:

```
stalk
├─ freeze     the hare is looking at us (its last-seen facing, within the arc)   → Stop
├─ bite       beside it, facing it                                              → Bite
├─ pounce     behind it, facing it, and a lunge lands beside it                 → Lunge that way
├─ check      we have walked `check_after` cells without seeing it               → face where we believe it is;
│                                                                                  hold once facing
├─ circle     we are off its rear line                                          → SneakTo a cell one notch round toward
│                                                                                  its rear, spiralling in by `spiral`
├─ approach   on the rear line, farther than `distance`                         → SneakTo the cell `distance` behind it
└─ watch                                                                        → face it, or hold if we already do
```

The check is Lexa's: a circling fox faces the way it walks, so every few cells it pivots to
see the hare is where it left it. If it is, the sighting resets the count and circling resumes;
if the fox looks straight at the believed cell within its near reach and sees nothing, `revise`
forgets the hare (the second of its two rules, beside standing on the cell), the stalk bids 0,
and the fox idles. `spiral` is the circle's pitch, Lexa's dial: 0 is a pure arc at the fox's current distance; a
positive angle tilts each swing that far inward off the tangent, so the fox closes as it
comes round (about 13% nearer per 30° swing at 15°), never nearer than `distance`. An
ambition that has lost its reason to run bids 0 and gets no boost, so a stalk whose prey was
forgotten is let go. All of it reads beliefs, so a hare that walked out of view is still
stalked to where it was last seen and, once the fox stands there and sees nothing, forgotten. `hexes.py` is the mind's
copy of the plane for the geometry: bearings, rotations, the twelve offsets. Watched live on
2026-09-27: the fox approached to three behind the hare and held; froze when the hare turned to
look; circled to the hare's new rear when it faced across; settled and faced it. Every decision
shows in the terminal view and the window as the path through the tree, `stalk › circle ›
round`, and the cell it wants as an outlined hex.

## Design

- **The believed world's growth**, one rule at a time, each watched in the window before the
  next: forgetting a thing whose believed cell is in view and empty (negative evidence);
  ageing and confidence; dead reckoning along a last course; a memory of cells seen and never
  seen. All inside `beliefs.py`; none touches the wire. exp-02's belief store is the model to
  draw on.
- **The mind's growth**, all proof-of-concept so far ("we'll flesh it out later"): the Sims'
  half, things in the believed world advertising what they offer and the body's motives
  weighting them, as the source of utilities; a search ambition for prey believed and lost;
  the hare's own ambitions (flee), which retire the scene's click; what `previous` feeds
  back (`Cancelled`, `Lost`); whether the mind defers to a reflex in hand, which needs a
  "by" on `Doing`, since the wire doesn't say whose intent it is.
- **Deferred words**: `Follow`, `Flee`, each a variant and a few lines in drive. The action
  words are in, one to one with `Action`; the tree does not use them yet.
- **A body's own memory in the game**, below reflexes and brainstem, so a reflex asks "not in
  what I believe" rather than "not in last tick's list"; the mind's believed world does not
  reach the reflexes. `TODO.md`.
- **Keys in the viewer**, if the one-shot `order` gets tiresome.
- **A shared Python package** for the wire once a second consumer (the cortex) arrives; the
  client lives in the midbrain until then.
- **Running `Sense` every n ticks**, if the cast's cost bites; every second tick keeps within
  what animal eyes do.
- **The scene's click-to-command** goes when the midbrain drives the hare or a selection module
  lands.
