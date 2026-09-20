# Brain hitlist

The agent architecture for the root project, and the order we're building it in.

Working document. Decisions land here so they stop being relitigated; open questions
stay visible so they stop being forgotten. Derived from a design conversation on
2026-09-20 — the reasoning is compressed to a line each here, not reproduced.

Not a spec and not a plan. `AGENTS.md` rules both of those out for the main project,
and that still holds: this is a decision register and a queue, and the design of each
bite is still agreed in chat before it is built.

---

## The shape

Four layers, split by **timescale**, because biology, caloric cost and cache locality
all independently arrive at the same partition.

| Layer | Rate | Biological analogue | Holds |
|---|---|---|---|
| Motor | 60 Hz | spinal reflex, cerebellum | current action, locomotion |
| Midbrain | ~10 Hz | colliculus, basal ganglia | salience, action selection |
| Cortex | ~1 Hz / offline | neocortex | the world model, drives |
| Hippocampus | write-fast, read-at-replay | hippocampal formation | episodes, cognitive map |

Reference rates this came from: spinal reflex 20–40 ms, basal ganglia action selection
100–200 ms, cortical deliberation 300–500 ms floor. Roughly an order of magnitude
apart, and 60/10/1 Hz lands on a 60 Hz game loop with no tuning.

---

## Settled

**Layering and scheduling**

1. Split components by **update frequency, not conceptual ownership**. One fat `Brain`
   component would drag the world model through cache 60 times a second to move a foot.
2. **Nothing blocks the frame.** Cortex may block on its own query; it runs as a task
   off the critical path.
3. **Reflexes preempt without arbitration.** The cord owns the hand for that moment. You
   drop the cup before it hurts; conscious pain arrives ~1 s later.
4. **Cortex proposes, it does not own the body.** An agent that waits for its own
   deliberation before yanking its hand back is the one that burns.
5. **Deliberation ticks synchronously across agents**, not staggered. Staggering is the
   usual game-AI advice for smoothing spikes, and it destroys the batch.

**Truth and belief**

6. The **corpus is the agent's memory, not the world.** Retrieval never touches truth.
7. **Perception is ingestion**, not retrieval — a lossy, partial, permission-limited
   crawler whose crawl budget is the vision cone. There is no fallback to origin.
8. The boundary is **enforced by query signature**. A system that didn't ask for world
   access cannot touch it, and the scheduler parallelises the two for free. This is
   strictly better than exp-02's import-hygiene convention.

**Storage and retrieval**

9. **GraphRAG-shaped hierarchical index**: broad-to-narrow ontological categories, with
   individually perceived entities as leaves. No embeddings — the ECS hands us the
   ontology that GraphRAG normally has to extract.
10. The tree is **multi-resolution memory**. Fine detail here, summaries further out.
    Memory becomes bounded rather than O(tiles). Matches the dorsoventral place-field
    scale gradient and multi-scale grid cells.
11. The tree is also **the anytime mechanism**. Iterative deepening: whatever tier the
    query reached when the deadline hit is a valid, blurrier answer.
12. Coarse-vs-fine is **the same split as the two visual pathways** — retinotectal fast
    and coarse, geniculostriate slow and fine. Not one signal at two speeds; two tiers
    of one index.
12a. **The ontology is data, not Rust types.** YAML, `include_str!`-ed like the locale
    catalogues. Not a style choice: Rust types cannot be traversed at runtime, and the
    whole tiered-index mechanism *is* runtime traversal. Enums would also make every new
    species a recompile plus every `match` arm.
12b. **Two axes, not one tree.** The taxonomy says what a thing *is*; **facets** are
    non-hierarchical tags saying what it's like — closer to tags on a paper than to a
    class hierarchy. 大人/子供 is a facet, because a child fox and a child human share it
    while sharing no branch.
12c. **Keys are kanji**, with display names coming from `assets/locales/` like all other
    text. Keeps "UI text is never a literal" intact, and sidesteps 物 and 者 both
    romanising to `mono`.
12d. **The root is 諸法** (shohō, all dharmas — every phenomenon, thing and event alike),
    with **諸法実相** named in the ontology file's header. 法 means *law* in ordinary
    modern Japanese, so the header is what disambiguates it as *dharma*; it does that
    once, and the key stays two characters in every parent pointer. Chosen over 存在
    (names the property, not the collection, and 存在者 collides with 者), 一切 (primes
    negative polarity — 一切ない, 一切禁止) and 万物 (contains 物, muddying the 物/事 cut).
    Abhidharma is the precedent: a worked hierarchical taxonomy of dharmas that already
    separates 色法 (matter) from 心不相応行法 (process and happening) — the same cut as
    物/事, with the awkward cases worked through.
12f. **The truth/belief cut is named 実相 / 仮.** What is in the engine — the entities,
    and the canonical taxonomy they are spawned from — is **諸法実相**, phenomena as they
    truly are. What is in a being's graph — its partial traversable copy, with its
    impressions and its instance leaves — is **仮諦**, provisional designation. The two
    are never the same object and the naming says so. 中諦 names the remaining slot: a
    debug view holding truth and construal at once without collapsing to either, as
    exp-02's `T` overlay did.
12g. **者 versus 物 is settled by Japanese grammar, not judgement.** いる for the
    animate (猫がいる, 神がいる, 妖怪がいる) and ある for the inanimate (木がある,
    石がある, 死体がある). 植物 takes ある, so plants sit under 物 and will never have
    senses or an intent. The corpse case is the useful one: dying moves a being from
    いる to ある, so death is a reclassification rather than a special case, and 付喪神
    is the same move in reverse.
12h. **幽 for the unseen**, not 妖怪 (too narrow, excludes 神) or 霊 (leans ghost). It
    classifies by realm — 幽世 against the manifest 顕世 — so 神 and 妖怪 sit together
    with neither filed under the other, and it leaves 顕/幽 available as a facet.
12i. **未知 is the name of a stopped traversal, not a node.** Category-neutral, so it
    reads correctly on both branches where 知らない物 would not, and it pairs with 既知.
    Building 3a-ii showed it should not be materialised: "stopping at 狼 and pulling
    狼's 未知" *is* the traversal stopping at 狼, and the generic impression hangs off
    that node in the being's 仮諦, where impressions live. The world tree has none to
    hang, so materialising it would add one dead key per node. The YAML header's claim
    that the loader adds them is superseded.
12j. **Rust domain types carry kanji** (実相, 仮諦), plumbing verbs stay English. A
    non-ASCII *module* name needs an explicit `#[path]`, since rustc will not infer a
    filename from one (E0754); a romaji filename does not help, because the error is
    about the identifier. Ontology nodes are never types anyway.
    Non-ASCII identifiers have been stable since Rust 1.53; verified compiling on 1.95
    with struct names, field names and arguments, no lints fired. Ontology nodes are
    never types at all — they are YAML keys and runtime strings — so 物 and 者 sharing
    a romanisation can never collide.
12e. **`諸法 → 未知` is the absolute floor** and can never be missing. "Something is
    there and I have no idea what" is where fight/flight/freeze/fawn live, and it is what
    guarantees the arena always has a bid.

**Selection**

13. **Utility AI is the midbrain, and it is a bidding arena, not a pipeline stage.**
    Everything that can propose bids; the max wins each tick; a floor bid always exists,
    so there is always a decision.
14. A late or missing answer is **one fewer bidder**, not a failure state. This is why a
    scored pool is the arbiter and not a behaviour tree, which stalls on a missing
    precondition.
15. **No epoch checking.** Staleness is priced, not policed: a stale bid rescored against
    current beliefs simply loses.
16. **Confidence multiplies the bid.** A shallow query yields a vaguer belief and a
    tentative action, so the agent prefers the familiar when it hasn't had time to think.
17. **Hysteresis on the incumbent action** or behaviour dithers. That knob is also the
    personality knob: low reads skittish, high reads stubborn.
18. The **double-take is emergent**, not authored — the arena re-running against better
    data. Nobody writes it.

**Learning**

19. **Efference copy**: an intent carries what it predicted would happen, so invalidation
    produces a matched predicted-vs-actual pair. Free labelled data, which is otherwise
    almost unobtainable in a game.
20. Bin the **command**, keep the **error**. (Corrected mid-conversation; the first
    version threw both away.)
21. Prediction error feeds **three timescales**: in-flight correction (~1 frame),
    per-thing model update (seconds–minutes, one float per believed thing), and a
    **futility integrator** (minutes–lifetime, one float per agent) that flips global
    disposition. The zebrafish glia result: accumulate evidence that actions are futile,
    then give up. Learned helplessness for the cost of a float.
22. **Hippocampus is an LSM-tree.** Episode buffer is the memtable, replay is compaction,
    the cortical model is the merged read-optimised structure. You cannot have cheap
    writes and good reads in one structure — which is also *why* it's a separate organ
    (catastrophic interference).
23. **Sleep is the batch window.** Consolidation happens offline, at 3am game time, when
    nothing is rendered and no one is watching. Evolution and RocksDB converged here.

**Senses and space**

24. **The senses are not one shape, and get no common trait.** Vision is a *pull* from
    the perceiver, recomputed and stateless. Hearing is a *push* from a source: an event
    is cast, and what arrives is then normalised by the listener's own polar pattern, so
    the cardioid is a receiver gain and not a propagation shape. Smell is a *field* — it
    emanates from a 物, spreads, drifts on the wind and persists after the source has
    gone, so its state does not live in the perceiver at all. They share geometry, and
    later a way to ask what lies between two points. Not an interface.
25. **Obstacles do not treat the senses alike.** Vision they block outright. Sound they
    attenuate, because it bends around: loudness falls with the shortest *unobstructed*
    path, which is what diffraction is. Smell they attenuate too, and it pools against
    them.
26. **Sound arrives with a bearing; smell arrives as a scalar.** One rustle tells a fox
    roughly where. A scent tells it only *here, this strong* — a direction has to be
    earned by moving and sampling again. Pouncing and quartering a field are the same
    difference, and neither has to be authored.
27. **Continuous bodies, hex-gridded senses.** Beings hold arbitrary positions; occlusion
    and propagation are computed on a grid underneath, so movement fidelity and sense
    cost are tuned separately. The grid is 1 shaku flat-to-flat, pointy-up.
    **Provisional.** exp-05 is laying the groundwork for a proper ri/cho/ken/shaku unit
    system, and this is a stand-in until that lands — written to be replaced, not
    extended. Every length in the crate is shaku; nothing converts to metres. Scale to
    keep in view: (√3/2) ≈ 0.866 shaku² a cell, ~5,000 cells over the 66 shaku (11 ken)
    starter ground, ~8,400 within the fox's 48 shaku vision.
28. **AI code lives under `gestalt_ai/`.** Everything brain-side — the layers, 仮諦, the
    arena, episodes, replay — goes there once it is built. Senses are body, not brain,
    and stay in `senses/`.

---

## The recognition loop

Perception **pushes**; the arena does not go looking. Per percept, per tick:

1. Senses produce a list of what was seen and heard. No identity attached yet.
2. Each percept is walked **down the being's own ontology** — its private, partial copy,
   not the world's — as deep as that being's knowledge allows.
3. At the deepest node it can reach it pulls either a **specific instance leaf** or that
   node's **未知** fallback.
4. What it found updates utility.
5. The arena runs and the being acts.

Worked examples, from the design conversation:

- `存在 → 者 → 生き物 → 動物 → 狼 → 未知` — *"I don't know which wolf this is, and it
  doesn't matter: I'm afraid of wolves."*
- `諸法 → 物 → 者 → 生き物 → 人間 → 玲子ちゃん` — everything this being feels about her
  specifically, keyed to one individual.
- A human who has never met a 妖怪 reaches only 者, pulls its 未知, and falls back
  on the generic responses: fight, flight, freeze, fawn.

Why this shape earns its place:

- **Knowledge is traversal depth.** Ignorance is not a missing entry or a special case —
  it is stopping higher up. Nothing ever has to ask "do I know this?"
- **Generalisation and specificity come from the same walk**, differing only in where it
  stops.
- **It degrades identically to the anytime query.** One tree, two independent reasons to
  stop early — out of *time*, or out of *knowledge* — and the same blurrier-but-valid
  answer either way. A structure solving a second problem it wasn't designed for.
- **The root always answers**, so the four Fs are the floor bid the arena is guaranteed.
- **Learning is extending a path**, not editing a record.

Corrects an earlier framing in this document's first draft, which had retrieval as a
pull driven by the active drive. The fast path is push. A deliberative pull may exist
later; it is not this loop.

---

## Deferred debts

Taken knowingly, for speed. Each one is cheap now and expensive to unpick later, so they
are written down rather than forgotten.

1. **Identity is given, not earned — a 仮諦 that cannot be wrong.** Beliefs key on
   `Entity`, so every 仮 node holds an infallible pointer back to its 実相, which defeats
   the point of the layer being provisional. Concretely, agents get perfect
   re-identification: they cannot mistake one villager for another, fail to recognise
   someone at range, or be fooled by a disguise. The mirror image of "must have been the
   wind" — unrealistically sharp instead of unrealistically dumb, and just as inert.
   Carving the world at its joints is most of what a world model *is*, and we have
   deleted that half. Revisit with a percept handle and a fallible association step.
2. **Tag matching has an affordance ceiling.** Without embeddings an agent can only want
   things a human labelled. "Something to sit on" will never retrieve an untagged log or
   low wall. This is the other half of the identity debt: cheap now, and precisely the
   part a world model would otherwise have to learn.
3. **Double-take resolution is undefined.** Does the fine answer cancel the coarse action,
   or does the coarse action complete and the correction show up as a second behaviour?
   These look completely different from outside; the second reads as embodied.

---

## Rejected, so we don't relitigate

- **Legibility theatre.** "It must have been the wind" is a belief state deliberately
  lobotomised; "flanking left" is a sound cue aimed at the player while the other NPCs'
  knowledge is unchanged. Neither involves an agent modelling anything. Building cheaply
  does not require building something intentionally stupid.
- **Learned latent world models.** Not for compute — because they are opaque (and here
  the model is the *content*, not a means to behaviour), nondeterministic (bugs must
  reproduce), and unauthorable.
- **Blocking the body on the brain.**
- **A single fat `Brain` component.**
- **A behaviour tree or FSM as the top-level arbiter.**
- **Porting exp-02 wholesale.** It was vibe-coded, a napkin test of a gestalt. Mine it,
  don't port it.

---

## Open

- **Drives.** Nothing is decided. This is the load-bearing gap: exp-02 spends ~394 lines
  on epistemics and 6 rows on motivation, so behaviour is a scripted tour rather than
  something generated. Population-level emergence needs agents wanting different things
  with different urgency and competing for the same affordances.
- **What an episode concretely is**, and what the consolidation policy rolls up or drops.
- **Growth and death.** Stated intent: they learn and grow, if they don't die first.
  Nothing designed yet.
- **Target population size**, which decides what "cheap" actually means.
- **Whether exp-03/exp-05's hierarchical hex addressing is reused** as the tree's
  address space.

- **The observer's relationship to the world** — embedded player, or a simulation watched
  and perturbed. Decides whether legibility must be diegetic.

---

## Bites

One at a time, smallest first, each one visible before the next is agreed. Order is a
proposal, not a commitment.

- [x] **0. Bodies, no brains.** A fox and a rabbit: `Vision` (one cone angle, three
      concentric range bands, acuity falling in steps) and `Hearing` (the microphone
      polar-pattern family — one number carries a rabbit's near-circle and a fox's
      forward lobe), both as data with pure scoring functions, plus a gizmo overlay.
      Nothing senses anything yet; these are the shapes. Tests assert the species claim
      — hunter and prey shaped oppositely by the same two senses — not the numbers,
      which are placeholder tuning.
- [x] **0a. Seeing the shapes.** `--zoom <metres>` on the one-shot capture. At the
      starting zoom the fields run off every edge, so the overlay had been rendered but
      never actually looked at.
- [x] **0b. One file per sense.** `senses/` as a parent with `vision.rs` and `hearing.rs`
      under it, each headed by what drives it. Pure refactor; the overlay renders
      pixel-identically.
- [x] **0c. The sense grid.** `src/hex.rs`: 1 shaku flat-to-flat, pointy-up, axial
      coordinates. World position to cell and back (cube rounding, so points near a
      corner land right), neighbours, distance, ranges. Drawn as a 6 ken patch around the
      camera's focus — ~4,000 cells, ~24,000 segments a frame — rather than the whole
      world, which at a cho of ground would be ~150,000 cells almost none of which are
      ever on screen. Terrain went to one cho at the same time.
      **Provisional** — see settled item 27; exp-05's unit system replaces it.
- [ ] **0d. Line of sight.** Vision gated by what stands in the way. Obstacles register
      as occluders; shadowcasting over the grid gives a visible set per perceiver that is
      computed once and reused for every candidate, rather than a ray per pair.
- [ ] **0e. Assert the tableau.** The starting arrangement is a predator watching prey
      that hasn't noticed — and the placeholder cube sits within 0.27 m of the fox-rabbit
      line, so once 0d lands the fox *cannot* see the rabbit, which is the more
      interesting starting position anyway. Currently eyeballed off a screenshot through
      a projection that makes ground angles hard to judge. It is a claim, so it should be
      a test.
- [ ] **0f. A legible overlay.** Each being's vision and hearing are drawn in one hue and
      overlap heavily, so which curve is which sense is guesswork; and the rabbit's
      vision — 2.5/5/8 m at low alpha, inside a 10 m hearing circle — is the faintest
      thing on screen while being its most interesting sense. Line-of-sight shadows and a
      scent field will make this worse, so it wants doing before they land, not after.
- [ ] **0g. Sound that bends.** A noise is an event cast from its maker; propagation over
      the grid attenuates it by shortest *unobstructed* path length, so a wall muffles
      rather than silences. Delivers an intensity and a bearing; the listener's existing
      polar pattern turns that into what it actually hears.
- [ ] **0h. 嗅覚.** Smell: emitted continuously by a 物, spreading and decaying over the
      grid and drifting with wind. The first sense with state of its own, and the first
      that makes the past readable in the present.
- [ ] **0i. Facing, visibly.** A slow idle turn, so the fields are seen to track the body
      rather than assumed to.
- [ ] **1. The layer skeleton.** Three systems at 60 / ~10 / ~1 Hz that do nothing
      interesting, plus a test proving each actually ticks at its rate under a varying
      frame time. Establishes `FixedUpdate` vs `Update` and the run conditions.
- [ ] **2. A body with an intent.** One agent, an `Intent` component, a motor system that
      executes it. No deciding yet. Something moves on screen.
- [ ] **3. The bidding arena.** Two or three hardcoded bidders, a floor bid, max wins,
      hysteresis. Still no memory — bids score off live world state.
- [x] **3a-i. The ontology YAML.** `assets/ontology/諸法.yaml` — 19 nodes, parses.
- [x] **3a-ii. The loader and traversal.** `src/諸法.rs`: `実相` as a resource, `path`,
      `is_a`, and `descend_while` as the seam the recognition loop plugs into.
- [x] **3b. Facets.** `属性` reserved on a node, inherited downward, validated against a
      declared registry. `食物連鎖: [捕食者, 被食者]` is the first real one. Beings now
      carry `種`, not `Fox`/`Rabbit` markers.
- [ ] **3b-ii. Instance facets.** A being carrying 子供 in its own right, rather than
      inheriting everything from its 種. 年齢 is declared and has nowhere to live yet.
- [ ] **3c. The recognition loop, shallow.** Percept in, walk the shared tree, pull the
      leaf, feed a bid. Every being still omniscient about the taxonomy.
- [ ] **3d. Private ontologies.** Each being gets its own partial copy, so traversal
      depth starts differing between beings and 未知 begins to matter.
- [ ] **4. The truth/belief cut.** Perception as ingestion into a flat belief store;
      retrieval systems lose world access. Prove the cut with a query signature.
- [ ] **5. The hierarchy.** Belief store becomes the tiered index. Coarse tier answers
      first; anytime depth becomes confidence, and confidence enters the bid score.
- [ ] **6. Efference copy and prediction error.** Intents carry a prediction; the error
      is captured rather than discarded.
- [ ] **7. Episodes.** An append-only buffer. Still no consolidation.
- [ ] **8. Replay.** Compaction into the cortical model during a quiet window.
- [ ] **9. Drives.** Only once there is something for them to be scored against.
