# All things

A brief on what a thing is: the two crates under `crates/all_things/`, `murabito_identity` and
`murabito_kinds`, as agreed in design on 2026-09-22 (the tree as types), 2026-09-24 (the number
every thing carries, the place every tangible thing must be given) and 2026-09-26 (the label, the
roster, the ontology). Implemented as described unless marked as design. The last section is the
procedure for adding a node.

## The shape of it

Anything in the world that is informative or interactive is a **thing**, and every thing is of a
**kind**. The kinds form one tree, 諸法 `AllThings` at its root, and a kind inherits everything its
ancestors have, the way classes do: a fox is a beast is an animal is a living thing is sentient is
tangible is a thing. Lights, cameras and UI are entities but not things: nothing in the tree
describes them, and they get none of what follows.

Three questions about a thing, answered in three places:

| Question | Answer | Where it comes from |
|---|---|---|
| What is it? | its `Kind`, a path in the tree | the node's file, `Kind = Kind::at(module_path!())` |
| Which one is it? | its `ThingId`, a number for life | minted at spawn by an observer on the root |
| Where is it? | `VoxelPosition` and `Facing` | given at spawn, beside the kind |

And what a thing *has*, its `Vision`, its `Locomotion`, its `Model`, is the kind's: written in the
node's `#[require]`, inherited down the tree, overridden by a more specific node.

**`murabito_identity`** owns the number and the label, and knows no tree. It depends on Bevy alone,
so anything may depend on it, the brainstem included, without learning that a fox exists.
**`murabito_kinds`** owns the tree, the roster that names its nodes, the `Ontology` that reads the
tree back from the world, and three observers. Only what spawns things depends on it.

## A thing's number: `ThingId`

Bevy's `Entity` is a handle for one run: a despawned thing's index is reused with a new generation,
and a save and a load hand out new ones. `ThingId` is ours: a serial number from one counter,
`NextThingId`, starting at 1, never reused, and stable across a save as long as the counter is saved
with the world. Whatever remembers a thing across ticks or saves keys on the id; whatever acts on it
now uses the handle. A `Sighting` carries both.

The id is stamped by an observer on `AllThings`, the root, so every thing gets one the moment it is
spawned and nothing else ever does. A spawner may give one itself, as a loader restoring a save
will, and what is given wins: the observer sees it and leaves the counter alone.

## A thing's name: `Kind`

`Kind` is a component holding a path: the module path of the file that defines the thing's node,
`murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox`. Every node's `require`
ends with `Kind = Kind::at(module_path!())`, and `module_path!()` expands to the path of the file it
is written in, so the file's place under `src/` is the label, with nothing to type and nothing to
fall out of step. A direct requirement wins over an inherited one, so a spawned fox is labelled
`…::fox`, never `…::beast`.

The path is the whole ancestry. `name()` gives the last segment, `fox`; `is_under(ancestor)` says
whether another kind's path is a proper prefix, so `…::fox` is under `…::beast`, under `…::animal`,
and not under itself. Whatever thinks about a thing can match on the leaf or on any ancestor, and
a future, fuzzier perception can name a thing by a prefix of its path, "some animal", "something
sentient", without the representation changing.

The label is the node, lowercase, the module. On the debug wire the same node's *type* shows as
`…::beast::fox::Fox`; the two differ by the last segment, and both are the tree.

## The tree

**A node is a unit component**, `pub struct Fox;`, and its `#[require(...)]` is both *extends* and
the members: the parent comes first, then what every thing of that kind has, then the label.
Spawning a node inserts the whole chain, so the node itself says what it has and the compiler holds
it to that. There are **tiers**, nodes with children (`Beast`), and **leaves**, nodes without
(`Fox`); both are spawnable, and both are labelled, though in practice only leaves are spawned.

**One node, one file, and the folders are the tree.** A tier is a file beside a folder of the same
name holding its children: `beast.rs` and `beast/fox.rs`. The tier's file declares its children as
modules (`pub mod fox;`) and re-exports them (`pub use fox::*;`), so every node is reachable as
`murabito_kinds::Fox`. English names, with the kanji in the file's first doc line: `//! 狐 Fox`.

**What a node carries.** A tier gives the defaults for everything under it; a leaf overrides what
differs. `Sentient` (有情, whatever takes いる) requires `Tangible`, a `Locomotion`, a `Vision` and an
`ActionQueue`, with placeholder numbers in consts in its own file; `Fox` requires `Beast`, its own
`Locomotion`, its own `Vision` and its `Model`. A species with no eyes requires `Vision::BLIND`.
`Model` is a path under `assets/`; an observer loads it the moment it lands. Exact numbers belong
to a node, never to the tree's shape: a `require` carries a value, so a kind's members are what
every instance shares.

**What is given at spawn.** Where a thing is cannot be in the tree, since a place is the instance's.
Every tangible thing (物, whatever has a place) must be spawned with a `VoxelPosition` and a `Facing`
beside its kind, and an observer on `Tangible` panics at spawn if either is missing, naming the
entity. If something has a position it has a facing; there is no default east. An intangible thing
(事) needs neither. A spawner may also give a particular `Model` variant, or a `ThingId` when
loading, and in every case what is given wins over what the kind would have inserted.

```rust
commands.spawn((Fox, VoxelPosition(start), Facing(Direction::ESE)));
```

**The roster** (`kinds/src/roster.rs`) names every node once, nested, so it reads as the tree it is:

```rust
tier::<AllThings>(vec![
    tier::<Intangible>(vec![]),
    tier::<Tangible>(vec![
        tier::<Sentient>(vec![
            tier::<Living>(vec![
                tier::<Animal>(vec![
                    tier::<Beast>(vec![leaf::<Bear>(), leaf::<Boar>(), …, leaf::<Fox>(), …]),
```

Rust can't enumerate types, so this is the one written list of the tree's members. Each entry
holds what can be done with its type without naming it again: register the component, spawn one,
put it on the debug wire. Everything that needs the nodes goes through it: the debug registration,
the tests' every-node spawn, and the ontology.

**The `Ontology`** is a resource `KindsPlugin` puts in the world: the tree as the running world has
it. Building it spawns every node of the roster once in a scratch world, reads its label, and finds
the deepest other node Bevy put on it, which is the `require` chain as the engine resolved it. That
parent must be the one the roster names, or the build panics naming the node, so the app never
starts with a tree that disagrees with itself. It offers `nodes()`, `root()`, `parent`, `children`,
`ancestors` and `render()`; `scripts/tree.sh` prints the rendering.

**Three statements of the tree, held to each other.** The `require` chain in each node's file, the
nesting of the roster, and the folders under `src/`, plus the labels, which are the folders spelled
as a path. Building the ontology checks the chain against the roster. The kinds tests check the
labels against the ontology (each node's path is its parent's path plus its own name) and the roster
against the files (a walk of `src/all_things/`, taking each node's parent to be the node its folder
is named after, compared pair for pair). Change any one of the four and the tests name the node.

## The observers

`KindsPlugin` runs three observers and nothing else; the kinds themselves are types and need no
systems.

- **`stamp_id`**, on `Add<AllThings>`: mints a `ThingId` unless the spawner gave one.
- **`check_placed`**, on `Add<Tangible>`: panics if the entity has no `VoxelPosition` or no `Facing`.
  Observers run inside the spawn's command flush, so a component given in the same bundle is already
  visible. Found the hard way: a tree without a facing stood at the world origin from PR #41 until
  #43, because `place` needs both.
- **`load_model`**, on `Add<Model>`: asks the asset server for the glTF scene and inserts a
  `WorldAssetRoot`. A `require` constructor is a plain function with no world in reach, so a kind
  can only name its file; loading is the crate's job.

The plugin adds `IdentityPlugin` itself if the app hasn't, since the counter must exist before the
first spawn.

## On the debug wire

Every node derives `Reflect` behind the `debug` feature, and the roster registers them all, so a
thing's component list on the wire is its whole chain by type path, and its `murabito_identity::Kind`
is the label as a bare string. The probe's `things` view shows both, and they always agree. The
debug registration is the roster, so a node on the tree is a node on the wire with no second list.

## Adding a node

The tests are written so that a half-done addition fails naming the node. Do the steps in any order
and run `cargo test -p murabito_kinds` at the end; below each step is what fails if it is skipped.

**A new leaf**, say a badger under `Beast`:

1. **The file**, `kinds/src/all_things/tangible/sentient/living/animal/beast/badger.rs`, in the
   folder of its parent. First line the kanji and the name, `//! 穴熊 Badger`. Then the struct, in
   the same shape as every other node:

   ```rust
   use bevy::prelude::*;
   use murabito_identity::Kind;

   use crate::{Beast, Model};

   /// 穴熊: the Japanese badger. <what makes it itself, and where its numbers come from>
   #[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
   #[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
   #[require(
       Beast,
       Model = Model("entity_models/living/animals/badger.glb"),
       Kind = Kind::at(module_path!()),
   )]
   pub struct Badger;
   ```

   The parent first. Then only what differs from the parent: a `Locomotion` or `Vision` of its own
   as a const in this file, a `Model` if it has one. The label last. Skip the label and the ontology
   build panics: "Badger has no Kind". Put the file in the wrong folder and the label test fails:
   "…::badger should sit in its parent's folder". Name the wrong parent in the `require` and the
   build panics: "Badger's require puts it under X, the roster under Beast".
2. **The parent's file**, `beast.rs`: `pub mod badger;` and `pub use badger::*;`, in name order with
   the others. Skip it and nothing compiles, since the roster can't see the type.
3. **The roster**, `roster.rs`: `leaf::<Badger>()` inside `tier::<Beast>(vec![…])`, in name order.
   Skip it and the files test fails: "left: the roster; right: the files under src/", with the badger
   only on the right.
4. **The model**, if it names one: the glTF under `assets/entity_models/`, through Git LFS. The test
   `every_model_a_kind_names_is_on_disk` fails otherwise, and the count in it goes up by one.
5. **The counts.** The tier tests count what is under each tier (`every_animal_is_one`); a new beast
   raises the beast count by one. Update the number and say why in its message.
6. **The wire** needs nothing: the roster registers it.
7. **Spawn one**, from a crate that depends on `murabito_kinds`, with a place and a facing beside
   it. A spawn without either panics at once.

**A new tier**, say `Rodent` between `Beast` and `Rat`:

1. `beast/rodent.rs` with the struct as above, requiring `Beast`, and a folder `beast/rodent/` that
   `rat.rs` moves into. The tier's file declares and re-exports its children, as `beast.rs` does.
2. `rat.rs` now requires `Rodent` instead of `Beast`. Its label changes by itself: it is the file's
   path.
3. `beast.rs` gains `pub mod rodent;` and `pub use rodent::*;` and loses the rat's lines.
4. The roster: `tier::<Rodent>(vec![leaf::<Rat>()])` where `leaf::<Rat>()` was.
5. The lib doc's diagram in `kinds/src/lib.rs`, if the tier is worth showing there; `scripts/tree.sh`
   is the one that can't drift.
6. The counts, as above: a tier is itself one more thing under every tier above it.

**Moving or removing a node** is the same edits in reverse, and the same tests say what was missed.
Whatever spawns the node (the scene, a loader) has to stop, or it fails to compile, which is the
point of only spawners depending on kinds.

## Design

- **Sentients gain a body and reflexes.** The AI work under way gives `Sentient` two more members:
  the brainstem's per-body component and a `Reflexes` repertoire, with each species filling its own
  repertoire and dials in its own `require`, the way it fills `Vision`. Per-instance dials are given
  at spawn beside the kind, so a jumpy fox is the fox's repertoire with one number changed.
- **Kanji in the tree.** The kanji lives only in each file's first doc line. If the rendering should
  show it, it becomes a member of the node, and the roster's rendering reads it.
- **Members for `Intangible`.** 事 has none yet; concepts, events and relations are design.
- **Facets.** What one thing can tell of another's kind by looking is perception's, not the tree's:
  a sighting names the seen thing's full path today, and a fuzzier eye will name a prefix of it.
