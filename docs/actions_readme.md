# Actions

A brief on how an entity does things that take time, as agreed in design. What is implemented is
marked; the rest is design. Movement's own rules are in `movement_readme.md`; this is the layer that
sequences them, and the accumulation they all share.

## The stack

Four layers, each a crate, each depending only downward:

| Layer | Crate | Directory | Knows |
|---|---|---|---|
| AI | later | | what an entity wants: decides, and issues actions |
| Actions | `murabito_actions` | `crates/action/actions/` | what it has been asked to do, in what order, and whether something is in flight |
| Mechanisms | `murabito_movement`, `murabito_attacks`, later block-breaking, archery, … | `crates/action/mechanisms/*/` | how one kind of thing is physically done: the intent, the effect, the cost |
| Progress | `murabito_progress` | `crates/action/progress/` | how far along a sustained action is, whatever it is |

The three below AI live together under `crates/action/`, a group directory rather than a crate.

A mechanism never sequences: it carries out one intent put on an entity (`Step`, `Turn`, `Bite`) and
removes it when done. The actions layer never accumulates: it reads whether something is in
flight and issues the next intent. Physical accumulation sits below AI, in one place.

## One accumulation bar: `Progress`

Implemented, in `murabito_progress`.

Every sustained action, whatever kind, accumulates in the same component. There is one
`Progress` per entity, because an entity does one sustained thing at a time; that is also the
actions layer's rule, and this is the physical fact behind it.

```rust
pub struct Progress { done: f32, of: f32, kind: Option<TypeId> }   // fields private
```

- **Units are the mechanism's, not time.** `of` is the cost of the thing in flight in whatever
  the mechanism measures: shaku for a step, degrees for a turn, hardness for a block, seconds for
  a bow. `done` accumulates in the same unit. `Progress` never knows what the unit is; only the
  mechanism that started the action does. Time comes in through the rate: a mechanism advances
  by `rate / tick_rate` each tick, and how many ticks an action takes falls out of that, rather
  than being stored or rounded anywhere.
- **`kind` records which intent started it** (`TypeId::of::<Step>()`: a compiler-assigned
  identity for a type, with no registry to keep). It is what makes carrying an overshoot safe.

The rules, written once and followed by every mechanism:

1. `start::<Step>(cost)` begins an action. If the last action was the same kind and finished on
   the previous tick, the leftover in `done` is kept; otherwise `done` starts at 0.
2. Each tick, `advance(amount) -> bool` adds to `done` and says whether it reached `of`.
3. On `true`, the mechanism does its effect, then calls `finish()`, which leaves the overshoot
   (`done - of`) in `done` and clears `of`.
4. A tick in which nothing was started and nothing is in flight zeroes `done`. One system in the
   progress crate does that, ordered after every mechanism, so an entity that stops and later
   starts again never begins with a head start.
5. `fraction()` is `done / of`, for whatever draws the bar; `in_flight()` and `kind_is::<Step>()`
   are for the actions layer, which then has no mechanism-specific "is it busy" to ask.

**Why carry the overshoot.** At 4 shaku/s and 64 ticks/s an edge step (cost 1) lands on tick 16
exactly, but a corner step (cost √3) passes its cost on tick 28 with 0.018 to spare. Discarding
that would make every corner step a little slow, forever. Keeping it starts the next step 0.018
in, so over a run of steps the average speed is exactly the stated one, while any single step
still lands on a whole tick. Keeping it only within a kind means a leftover 0.018 shaku after a
step never becomes 0.018 degrees of a turn.

**Why not ticks.** Measuring in ticks would make the bar uniform, but every cost would have to be
rounded to whole ticks up front, which is the corner-step error above with no way to cancel it.

## Intents

Implemented, in `murabito_movement` and `murabito_attacks`. A mechanism exposes its actions as
**intent components**: put a `Step { direction, gait, reach }`, a `Turn(Direction)` or a `Bite`
on an entity and the matching `FixedUpdate` system carries it out over ticks, through
`Progress`, then removes it. One intent at a time; the mechanism enforces its own physical rules
(a step lands facing by its way, at most one notch from where it was; a turn goes a notch at a
time, the short way round). `murabito_movement` is the first mechanism; its rules are in
`movement_readme.md`. `murabito_attacks` is the second, a stub: a `Bite` takes a quarter of a
second on the bar and bites nothing, holding the word's place until there are jaws, targets and
hurt.

An intent is the mechanism's own shape, with every knob a body can physically vary: a step's
gait and reach. Nothing above the mechanism sees those knobs. The layer above speaks in
**words**, and each word fixes its knobs under the hood.

## The queue, and the words

Implemented, in `murabito_actions`. `ActionQueue` is a component holding a queue of `Action`s,
first to last. It is a queue of actions and nothing more: anything may push onto it, an
instinct, a social pull, a player's command, a planner, and nothing in it says who did or why.

`Action` is the vocabulary of everything above this crate. Ten words, each with a direction
except the last:

| Word | Under the hood | Only |
|---|---|---|
| `Walk(d)` | one voxel at walking pace, turning first if need be | any way |
| `Jog(d)` | a walk at twice the pace | any way |
| `Sprint(d)` | a walk at three times the pace | any way |
| `Sneak(d)` | a walk at half the pace | any way |
| `Face(d)` | a turn, without moving | |
| `Sidestep(d)` | one voxel at a walk, without turning; lands facing orthogonal to the way | lateral |
| `Backstep(d)` | one voxel at a walk, without turning; lands facing away from the way | rear |
| `Recoil(d)` | a backstep at a jog | rear |
| `Lunge(d)` | two voxels at a sprint, landed in one go | forward |
| `Bite` | a bite at whatever is in the cell faced (nothing yet) | across a face: refused when the body faces a corner direction |

A word that names a way is only that way of the body's facing (`Way::of`, in
`movement_readme.md`), and a bite reaches only across a face (Lexa, 2026-09-28: "bites are
faces, not corners"). One asked the wrong way, or a bite asked facing a corner, is
**refused** (`Refusal`, with its reason: "not lateral", "not across a face"): dropped from the
queue with a warning, never bent into something else, since whoever asked has it wrong and
should hear so. `Action::check(facing)` is the same rule, askable before pushing, so the
brainstem can end a wrong word as `Refused` and tell the mind rather than the log.

One `FixedUpdate` system, `issue`, runs before the mechanisms. For a body with nothing in flight
(no intent on it, and `Progress` idle) it takes the action at the head and issues the intent it
needs next; the action is dropped once its last intent is out. That is where sequencing lives:

- `Face(d)` issues a `Turn(d)`, and is done.
- A pace word, when the body faces within one notch of `d`, issues a `Step` at its gait, and is
  done: the landing takes the last notch for free.
- A pace word otherwise issues a `Turn` to one notch short of `d` on the near side, and stays
  at the head; on the tick after that turn ends it is looked at again, and is now a step. That
  is the turn-then-step rule, in one place.
- A way word issues its `Step` and is done, or is refused; it never turns first.
- `Bite` issues a `Bite`, and is done.
- Nothing is issued while something is in flight, whatever kind it is.

The queue also keeps the action **underway**: the one whose last intent was issued, off the
queue and not yet landed, so the brainstem's snapshot can say which way the step in flight
goes and a mind can plan from where it lands. While a pace word's turn is in flight the word
is still at the head, and nothing is underway. `truncate(n)` keeps the first `n` waiting and
drops the rest, for a path amended from outside; what is in flight finishes.

Whatever pushes onto a queue runs in `AskingSet`, which `issue` follows, so an action asked
for on a tick is looked at on that tick rather than the next depending on which system the
scheduler happened to run first. Running before the mechanisms means an intent issued on a tick starts on that tick, so a `Walk` that
needs no turn lands exactly when a bare `Step` would. The state machine is implicit: the state is
the action at the head plus what is in flight, read from the components rather than kept in an
enum. A new word is a new `Action` variant here, fixing its knobs on an intent that exists; a
new kind of physics is a new mechanism crate below as well. The queue does not change either
way.

## Open questions

- **Cost of a turn while stepping:** the free one-notch turn on landing is a movement rule; does
  any other action get a free adjustment like it?
- **The cell a lunge skips:** a body is never in it. Whether it must be free is the same
  question as the corner-obstruction rule for ordinary steps, still open in `movement_readme.md`.
