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

---

## Deferred debts

Taken knowingly, for speed. Each one is cheap now and expensive to unpick later, so they
are written down rather than forgotten.

1. **Identity is given, not earned.** Beliefs key on `Entity`, so agents get perfect
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

- [ ] **1. The layer skeleton.** Three systems at 60 / ~10 / ~1 Hz that do nothing
      interesting, plus a test proving each actually ticks at its rate under a varying
      frame time. Establishes `FixedUpdate` vs `Update` and the run conditions.
- [ ] **2. A body with an intent.** One agent, an `Intent` component, a motor system that
      executes it. No deciding yet. Something moves on screen.
- [ ] **3. The bidding arena.** Two or three hardcoded bidders, a floor bid, max wins,
      hysteresis. Still no memory — bids score off live world state.
- [ ] **4. The truth/belief cut.** Perception as ingestion into a flat belief store;
      retrieval systems lose world access. Prove the cut with a query signature.
- [ ] **5. The hierarchy.** Belief store becomes the tiered index. Coarse tier answers
      first; anytime depth becomes confidence, and confidence enters the bid score.
- [ ] **6. Efference copy and prediction error.** Intents carry a prediction; the error
      is captured rather than discarded.
- [ ] **7. Episodes.** An append-only buffer. Still no consolidation.
- [ ] **8. Replay.** Compaction into the cortical model during a quiet window.
- [ ] **9. Drives.** Only once there is something for them to be scored against.
