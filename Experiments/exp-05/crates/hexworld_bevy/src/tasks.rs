//! Generation and meshing on Bevy's background task pool. Chunks are pure functions of
//! the config and the key, so a job needs nothing from the main thread and can finish in
//! any order.
//!
//! A chunk's *generation* job (`ChunkJob`, below) always runs async — that was already
//! true before this module's handover machinery existed. What is new is that a chunk's
//! *reveal* can also need another chunk re-meshed first (its parent omitting the new
//! chunk's cell, a same-level neighbour ceding it a guest cell, or the chunk itself
//! omitting an already-shown child) — and a cho chunk's re-mesh alone costs ~18 ms in
//! release, well past doing it on the main thread during a burst of arrivals (measured:
//! 150–660 ms frame spikes during sustained panning; see `task-12b-report.md`). So a
//! chunk whose arrival or departure needs any of that is *held back* — its entity is not
//! spawned, or despawned, until every re-mesh its handover needs has finished on the pool
//! — and then everything lands together, in one frame: see `Handovers` and
//! `process_handovers`.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use hexworld::{mesh::MeshData, Chunk, ChunkKey, Hex, WorldConfig};

use crate::{entities, entities::Shown, HexWorld};

pub struct JobOutput {
    pub key: ChunkKey,
    pub chunk: Chunk,
    pub mesh: MeshData,
}

#[derive(Component)]
pub struct ChunkJob(pub Task<JobOutput>);

/// Start generating the chunks the store asked for, up to the in-flight cap. Each job
/// meshes with the chunk's *current* omissions (always empty at this point in practice —
/// a chunk only ever gains omissions once it is shown, and this is the first time it is
/// requested at all — but read fresh here rather than assumed, so a reload after a full
/// unload still starts from whatever `Shown` says now).
pub fn spawn_jobs(
    commands: &mut Commands,
    world: &mut HexWorld,
    shown: &Shown,
    to_load: Vec<ChunkKey>,
) {
    let cap = world.store().settings().max_in_flight;
    let running = world.store().in_flight_count();
    let pool = AsyncComputeTaskPool::get();
    for key in to_load.into_iter().take(cap.saturating_sub(running)) {
        let cfg: WorldConfig = *world.store().config();
        let omitted = shown.omitted_for(key).clone();
        world.store_mut().begin_load(key);
        let task = pool.spawn(async move {
            let chunk = hexworld::chunk::generate(&cfg, key);
            let mesh = hexworld::mesh::mesh_chunk(&cfg, &chunk, &omitted);
            JobOutput { key, chunk, mesh }
        });
        commands.spawn(ChunkJob(task));
    }
}

/// One background re-mesh, as part of a handover: `key`'s mesh, recomputed with the cells
/// its handover adds to (a load) or removes from (an unload) its omissions.
struct RemeshTask {
    key: ChunkKey,
    task: Task<MeshData>,
}

fn spawn_remesh(
    pool: &AsyncComputeTaskPool,
    cfg: WorldConfig,
    key: ChunkKey,
    chunk: Chunk,
    omitted: HashSet<Hex>,
) -> RemeshTask {
    let task = pool.spawn(async move { hexworld::mesh::mesh_chunk(&cfg, &chunk, &omitted) });
    RemeshTask { key, task }
}

/// How a handover's snapshot of a target's entity, taken at promotion time, compares to
/// what `Shown` says right now. A target chunk can unload — or unload and reload with a
/// fresh entity — while a handover's background re-mesh for it is in flight; treating
/// those two outcomes the same (task-12b-report.md fix round 1's `target_is_still_valid`)
/// is itself a bug (fix round 2): a target that is genuinely `Gone` contributes nothing
/// further and is safely dropped, but a `Reloaded` target still needs its cells re-meshed
/// against its *current* state — discarding it instead silently drops the handover's work
/// for that target, leaving the two chunks permanently disagreeing about who draws the
/// cell (a doubled surface on the load side, a hole on the unload side). See
/// `replan_reloaded_targets` and `apply_finished_loads` / `apply_finished_unloads`.
#[derive(PartialEq, Eq, Debug)]
enum TargetStatus {
    /// Still exactly the entity this was planned against.
    Unchanged,
    /// Unloaded and not shown at all any more.
    Gone,
    /// Unloaded and reloaded with a different (but real) entity.
    Reloaded(Entity),
}

fn target_status(shown: &Shown, target: ChunkKey, entity_snapshot: Entity) -> TargetStatus {
    match shown.entity(target) {
        Some(current) if current == entity_snapshot => TargetStatus::Unchanged,
        Some(current) => TargetStatus::Reloaded(current),
        None => TargetStatus::Gone,
    }
}

/// After every one of a handover's originally-spawned re-meshes has landed, check whether
/// any target reloaded with a fresh entity in the meantime (as opposed to unloading and
/// staying gone — see `TargetStatus`). Re-plans and re-spawns a re-mesh for each one found,
/// against its *live* current omissions, mutating `targets` (the stale entity snapshot is
/// replaced with the fresh one) and dropping any now-stale finished result for it from
/// `results`. Returns whether it queued at least one new re-mesh, in which case the caller
/// must treat the whole handover as still waiting — exactly like an original re-mesh that
/// has not landed yet — rather than applying anything this frame.
#[allow(clippy::too_many_arguments)]
fn replan_reloaded_targets(
    world: &HexWorld,
    shown: &Shown,
    cfg: &WorldConfig,
    pool: &AsyncComputeTaskPool,
    targets: &mut HashMap<ChunkKey, (Entity, HashSet<Hex>)>,
    results: &mut Vec<(ChunkKey, MeshData)>,
    remeshes: &mut Vec<RemeshTask>,
    // Union for a load handover's `omit` (cells to *add*), set difference for an unload
    // handover's `restore` (cells to *remove*) — the two are otherwise identical.
    combine: impl Fn(&HashSet<Hex>, &HashSet<Hex>) -> HashSet<Hex>,
) -> bool {
    let mut replanned = false;
    for (&target, (entity_snapshot, cells)) in targets.iter_mut() {
        let TargetStatus::Reloaded(current) = target_status(shown, target, *entity_snapshot) else {
            continue; // unchanged, or gone — the final apply step handles both correctly
        };
        // A chunk that reloaded must be loaded again by definition, but a burst of churn
        // could in principle have it unload a *second* time before this very check reads
        // it — in which case there is nothing to re-mesh yet either; leave the stale
        // snapshot as-is and let the *next* round see it as `Gone` and drop it cleanly.
        let Some(chunk) = world.store().chunk(target) else {
            continue;
        };
        let final_omitted = combine(shown.omitted_for(target), cells);
        remeshes.push(spawn_remesh(
            pool,
            *cfg,
            target,
            chunk.clone(),
            final_omitted,
        ));
        results.retain(|(t, _)| *t != target);
        *entity_snapshot = current;
        replanned = true;
    }
    replanned
}

fn union(a: &HashSet<Hex>, b: &HashSet<Hex>) -> HashSet<Hex> {
    a.union(b).copied().collect()
}

fn difference(a: &HashSet<Hex>, b: &HashSet<Hex>) -> HashSet<Hex> {
    a.difference(b).copied().collect()
}

/// After every one of a load handover's currently in-flight re-meshes has landed (and
/// `replan_reloaded_targets` has already handled any *existing* target whose entity
/// changed), check whether `plan_load_handover`, recomputed fresh right now, wants
/// anything this handover does not yet have queued up: `key` itself omitting more (an
/// already-shown grandchild, or a guest cell whose owner only became shown since this was
/// last checked), or a target — a parent or same-level neighbour — that either was not a
/// target at all yet (it only became shown after this handover was promoted, or during an
/// earlier round of this very reconciliation) or, defensively, wants more cells than it
/// already has queued (the increment for an existing target is fixed by geometry and
/// should never actually grow in practice, but this is checked the same way regardless).
/// Spawns whatever re-meshes are newly needed, drops any now-stale finished result they
/// replace, and updates `self_omit`/`omit` to match. Returns whether it queued at least
/// one, in which case the caller must keep waiting — exactly like an original re-mesh
/// that has not landed yet — rather than apply anything this frame.
#[allow(clippy::too_many_arguments)]
fn reconcile_load_plan(
    world: &HexWorld,
    shown: &Shown,
    cfg: &WorldConfig,
    pool: &AsyncComputeTaskPool,
    key: ChunkKey,
    child_chunk: &Chunk,
    self_omit: &mut HashSet<Hex>,
    omit: &mut HashMap<ChunkKey, (Entity, HashSet<Hex>)>,
    results: &mut Vec<(ChunkKey, MeshData)>,
    remeshes: &mut Vec<RemeshTask>,
) -> bool {
    let fresh_plan = entities::plan_load_handover(cfg, shown, key);
    let mut needs_another_round = false;

    let fresh_self = fresh_plan.get(&key).cloned().unwrap_or_default();
    if !fresh_self.is_subset(self_omit) {
        *self_omit = union(self_omit, &fresh_self);
        results.retain(|(t, _)| *t != key);
        remeshes.push(spawn_remesh(
            pool,
            *cfg,
            key,
            child_chunk.clone(),
            self_omit.clone(),
        ));
        needs_another_round = true;
    }

    for (&target, fresh_cells) in &fresh_plan {
        if target == key {
            continue;
        }
        let already_covered = omit
            .get(&target)
            .is_some_and(|(_, cells)| fresh_cells.is_subset(cells));
        if already_covered {
            continue;
        }
        // A brand new target (the common case reaching here), or — defensively — one
        // whose increment grew. Either way, `plan_load_handover` only proposes an
        // already-shown target, but that can still have changed identity since; drop it
        // rather than acting on stale data (the next reconciliation, or the final apply
        // step, resolves it correctly from fresh state).
        let Some(chunk) = world.store().chunk(target) else {
            continue;
        };
        let Some(entity) = shown.entity(target) else {
            continue;
        };
        let existing_cells = omit
            .get(&target)
            .map(|(_, cells)| cells.clone())
            .unwrap_or_default();
        let added = union(&existing_cells, fresh_cells);
        let final_omitted = union(shown.omitted_for(target), &added);
        remeshes.push(spawn_remesh(
            pool,
            *cfg,
            target,
            chunk.clone(),
            final_omitted,
        ));
        results.retain(|(t, _)| *t != target);
        omit.insert(target, (entity, added));
        needs_another_round = true;
    }

    needs_another_round
}

/// Poll every `RemeshTask` in `tasks`, removing the ones that finished. Returns their
/// results; a non-empty leftover in `tasks` means the handover is still waiting.
fn poll_remeshes(tasks: &mut Vec<RemeshTask>) -> Vec<(ChunkKey, MeshData)> {
    let mut finished = Vec::new();
    tasks.retain_mut(
        |remesh| match block_on(future::poll_once(&mut remesh.task)) {
            Some(data) => {
                finished.push((remesh.key, data));
                false
            }
            None => true,
        },
    );
    finished
}

/// A child chunk's arrival, held back until every chunk its being shown changes — its
/// parent, any same-level neighbour ceding it a guest cell, and itself if its own
/// omissions grew after its generation job started — has been re-meshed off the main
/// thread. Then all of it, the mesh swaps and the child's own entity, is applied together,
/// so a doubled surface (the parent's stale mesh still drawing the child's cell) never
/// reaches the screen even for one frame.
struct LoadHandover {
    child: JobOutput,
    /// Cells the child itself must newly omit (an already-shown child of its own, or a
    /// guest cell whose true owner is already on screen) — applied unconditionally once
    /// `show_child` gives the child a real entity, since there is no earlier incarnation
    /// of it to have gone stale.
    self_omit: HashSet<Hex>,
    /// Every *other* affected chunk (parent, or a same-level neighbour) that must newly
    /// omit one of the child's cells, and the `Entity` it had at the moment this was
    /// planned (or last re-planned — see `replan_reloaded_targets`). Applying this later
    /// only ever touches a target that still has exactly that entity; see
    /// `apply_finished_loads`.
    omit: HashMap<ChunkKey, (Entity, HashSet<Hex>)>,
    remeshes: Vec<RemeshTask>,
    /// Re-meshes that already finished, in case they land across more than one frame — a
    /// result is kept here (never applied) until every one of them has.
    results: Vec<(ChunkKey, MeshData)>,
}

/// A chunk's departure, held back the same way: every same-level neighbour and the parent
/// that must draw its cell again is re-meshed off the main thread first, then the restores
/// and the chunk's own despawn are applied together.
struct UnloadHandover {
    key: ChunkKey,
    /// The entity as of when this handover was queued. A reload racing `key` back in
    /// before this applies would give it a fresh entity in `Shown`; applying this handover
    /// only ever despawns this exact snapshot, never a newer one (see
    /// `apply_finished_unloads`).
    entity: Entity,
    /// Cells each affected chunk must restore, and the `Entity` it had when this handover
    /// was planned — same staleness guard as `LoadHandover::omit`, and for the same
    /// reason: a neighbour or parent can itself unload (or unload and reload) while this
    /// handover's re-mesh is still in flight.
    restore: HashMap<ChunkKey, (Entity, HashSet<Hex>)>,
    remeshes: Vec<RemeshTask>,
    /// Re-meshes that already finished, in case they land across more than one frame — a
    /// result is kept here (never applied) until every one of them has.
    results: Vec<(ChunkKey, MeshData)>,
}

/// How many handovers — a child's arrival or a chunk's departure, each possibly
/// re-meshing several neighbours — may have work in flight on the pool at once. Bounds how
/// much of a burst (many chunks crossing the load/unload boundary in the same stretch of
/// frames) lands on the pool together; anything beyond the cap simply waits its turn in
/// the queue, which — per the one-frame rule this module exists to keep — is invisible on
/// screen: a child that is not shown yet is exactly as correct as one that has not loaded
/// yet, and a chunk mid-unload just keeps drawing what it was already drawing.
const MAX_ACTIVE_HANDOVERS: usize = 3;

/// Chunk arrivals and departures whose handover needs more than showing/despawning the
/// chunk itself, queued until they can run without racing another active handover that
/// targets the same chunk, then held until every re-mesh they started lands — see
/// `LoadHandover` / `UnloadHandover` and `process_handovers`.
#[derive(Resource, Default)]
pub struct Handovers {
    queued_loads: VecDeque<JobOutput>,
    queued_unloads: VecDeque<(ChunkKey, Entity)>,
    active_loads: Vec<LoadHandover>,
    active_unloads: Vec<UnloadHandover>,
}

impl Handovers {
    /// Queue a chunk whose departure needs a handover (`key` currently has an entity).
    /// Called from `drive_store`, which still runs every unload through `Shown` first, so
    /// only a key that is actually shown is ever queued here.
    pub fn queue_unload(&mut self, key: ChunkKey, entity: Entity) {
        self.queued_unloads.push_back((key, entity));
    }

    /// Whether `key` is a child chunk currently being held back by a load handover — its
    /// generation job has finished, but it is waiting (queued or already re-meshing on the
    /// pool) for its parent/neighbours to be re-meshed before it can be shown. Read-only
    /// introspection for tests: the one-frame rule is exactly "a key for which this is
    /// true must never also have an entity in `Shown`" (see
    /// `tests/handover.rs::a_child_is_never_shown_while_its_load_handover_is_pending`).
    pub fn is_pending_load(&self, key: ChunkKey) -> bool {
        self.queued_loads.iter().any(|output| output.key == key)
            || self
                .active_loads
                .iter()
                .any(|handover| handover.child.key == key)
    }

    /// Every child chunk currently held back by a load handover — see `is_pending_load`.
    pub fn pending_load_keys(&self) -> Vec<ChunkKey> {
        self.queued_loads
            .iter()
            .map(|output| output.key)
            .chain(self.active_loads.iter().map(|handover| handover.child.key))
            .collect()
    }

    /// Every chunk currently held back by an unload handover (queued or actively
    /// re-meshing), still keyed by the *departing* chunk itself rather than its targets.
    /// Read-only introspection for tests — see
    /// `tests/handover.rs::growing_the_window_back_after_a_shrink_leaves_no_duplicate_or_orphaned_entities`,
    /// which uses this to know when a real backlog of unloads has built up before racing a
    /// reload against it.
    pub fn pending_unload_keys(&self) -> Vec<ChunkKey> {
        self.queued_unloads
            .iter()
            .map(|(key, _)| *key)
            .chain(self.active_unloads.iter().map(|handover| handover.key))
            .collect()
    }

    /// Whether nothing is queued or active on either side. A "settled" test helper that
    /// only checks the store's own bookkeeping (`in_flight_count`/`loaded_keys`) can read
    /// settled while a handover is still queued or mid-re-mesh — the store considers a
    /// load "loaded" and an unload already gone from `loaded` well before either one's
    /// handover actually finishes; see `tests/common/mod.rs::run_until_fully_settled` and
    /// `tests/handover.rs`'s local `world_is_fully_settled`, both gated on this too (fix
    /// round 2, task-12b-report.md — the flake this closes: a stale queued unload racing a
    /// reload past a settle check that never looked at it).
    pub fn is_idle(&self) -> bool {
        self.queued_loads.is_empty()
            && self.queued_unloads.is_empty()
            && self.active_loads.is_empty()
            && self.active_unloads.is_empty()
    }

    fn active_targets(&self) -> HashSet<ChunkKey> {
        let mut targets = HashSet::new();
        for handover in &self.active_loads {
            targets.extend(handover.omit.keys().copied());
        }
        for handover in &self.active_unloads {
            targets.extend(handover.restore.keys().copied());
        }
        targets
    }

    fn active_count(&self) -> usize {
        self.active_loads.len() + self.active_unloads.len()
    }
}

/// Take finished generation jobs and store each chunk. A chunk whose arrival changes
/// nothing else — the common case: it loads into territory nothing else is drawing — is
/// shown immediately, right here. One whose arrival requires re-meshing another chunk (or
/// itself) is headed to `Handovers` instead, and shown later by `process_handovers` once
/// that work lands off the main thread.
#[allow(clippy::too_many_arguments)]
pub fn collect_finished_jobs(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    mut shown: ResMut<Shown>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::GroundMaterialHandle>>,
    mut handovers: ResMut<Handovers>,
    mut jobs: Query<(Entity, &mut ChunkJob)>,
) {
    let now = time.elapsed_secs_f64();
    let cfg = *world.store().config();
    for (entity, mut job) in &mut jobs {
        let Some(output) = block_on(future::poll_once(&mut job.0)) else {
            continue;
        };
        commands.entity(entity).despawn();
        let key = output.key;
        let plan = entities::plan_load_handover(&cfg, &shown, key);
        if plan.is_empty() {
            // The common case: showing this chunk changes nothing else, so commit and
            // show it right away — `insert` is the same gate it has always been for a
            // chunk the window stopped wanting while its job ran.
            if world.store_mut().insert(output.chunk, now) {
                show_child(
                    &mut commands,
                    &mut meshes,
                    material.as_deref(),
                    &mut shown,
                    &cfg,
                    key,
                    &output.mesh,
                );
            }
        } else {
            // Showing this chunk changes another chunk's mesh too. Hold it — and its
            // commit to the store — until every re-mesh that needs lands; see
            // `process_handovers`. Deliberately *not* calling `insert` yet: `in_flight`
            // stays set for `key`, so the store does not call it "loaded" (and a test's
            // "settled" check does not call the world settled) until it is genuinely
            // shown, not merely generated.
            handovers.queued_loads.push_back(output);
        }
    }
}

/// Spawn `key`'s entity and record it in `Shown` — the reveal half of a load, shared by
/// the fast (no-handover) path and a handover's apply step.
fn show_child(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Option<&crate::GroundMaterialHandle>,
    shown: &mut Shown,
    cfg: &WorldConfig,
    key: ChunkKey,
    mesh: &MeshData,
) {
    let Some(material) = material else {
        return;
    };
    let chunk_entity =
        entities::spawn_chunk_entity(commands, meshes, material.0.clone(), key, mesh, cfg);
    shown.insert_entity(key, chunk_entity);
}

/// Promote queued loads into active handovers (up to the cap, skipping anything that
/// would race an already-active target), poll every active handover's re-meshes, and
/// apply the ones that finished — all in one frame per handover once it is complete, which
/// is what keeps the one-frame rule (see the module doc).
pub fn process_handovers(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    mut shown: ResMut<Shown>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::GroundMaterialHandle>>,
    mut handovers: ResMut<Handovers>,
) {
    let now = time.elapsed_secs_f64();
    let cfg = *world.store().config();
    let pool = AsyncComputeTaskPool::get();

    promote_unloads(&mut commands, &mut handovers, &world, &shown, &cfg, pool);
    promote_loads(
        &mut commands,
        &mut handovers,
        &mut world,
        &mut shown,
        &mut meshes,
        material.as_deref(),
        &cfg,
        pool,
        now,
    );

    apply_finished_unloads(
        &mut commands,
        &world,
        &mut shown,
        &mut meshes,
        &mut handovers,
    );
    apply_finished_loads(
        &mut commands,
        &mut world,
        &mut shown,
        &mut meshes,
        material.as_deref(),
        &mut handovers,
        now,
    );
}

#[allow(clippy::too_many_arguments)]
fn promote_loads(
    commands: &mut Commands,
    handovers: &mut Handovers,
    world: &mut HexWorld,
    shown: &mut Shown,
    meshes: &mut Assets<Mesh>,
    material: Option<&crate::GroundMaterialHandle>,
    cfg: &WorldConfig,
    pool: &AsyncComputeTaskPool,
    now: f64,
) {
    while handovers.active_count() < MAX_ACTIVE_HANDOVERS {
        let Some(output) = handovers.queued_loads.pop_front() else {
            break;
        };
        // Abandoned while queued (the window shrank and this chunk is no longer wanted):
        // commit-and-reject, purely to release the store's in-flight slot for this key
        // (see `ChunkStore::insert`) — nothing is shown.
        if !world.store().is_requested(output.key) {
            world.store_mut().insert(output.chunk, now);
            continue;
        }
        let plan = entities::plan_load_handover(cfg, shown, output.key);
        if plan.is_empty() {
            // Something changed since this was queued (e.g. its would-be parent unloaded
            // in the meantime) and it no longer needs a handover at all — show it now,
            // the same as the fast path in `collect_finished_jobs`, and move on to the
            // next queued item rather than getting stuck retrying this one forever.
            if world.store_mut().insert(output.chunk.clone(), now) {
                show_child(
                    commands,
                    meshes,
                    material,
                    shown,
                    cfg,
                    output.key,
                    &output.mesh,
                );
            }
            continue;
        }
        let targets: HashSet<ChunkKey> = plan.keys().copied().collect();
        if !targets.is_disjoint(&handovers.active_targets()) {
            // Would race an already-active target: not ready to promote this frame. Put
            // it back and stop — anything behind it in the queue can only be younger, so
            // retrying it out of order would not help it land any sooner.
            handovers.queued_loads.push_front(output);
            break;
        }
        let mut remeshes = Vec::with_capacity(plan.len());
        let mut self_omit: HashSet<Hex> = HashSet::new();
        let mut omit: HashMap<ChunkKey, (Entity, HashSet<Hex>)> =
            HashMap::with_capacity(plan.len());
        for (&target, added) in &plan {
            if target == output.key {
                // The child itself: always available (this handover generated it), and
                // has no entity yet, so nothing to snapshot — see `LoadHandover::self_omit`.
                remeshes.push(spawn_remesh(
                    pool,
                    *cfg,
                    target,
                    output.chunk.clone(),
                    added.clone(),
                ));
                self_omit = added.clone();
                continue;
            }
            // A partner target's store data (or its `Shown` entity) can already be gone
            // by the time we get here — `ChunkStore::update` removes a chunk's data
            // synchronously, ahead of this system, the moment it is requested to unload,
            // while the entity itself lingers until that chunk's *own* unload handover
            // applies. Either way, drop the target from this handover entirely rather
            // than applying half of it: its own unload handover (if any) is what
            // correctly resolves it.
            let Some(chunk) = world.store().chunk(target) else {
                continue;
            };
            let Some(entity) = shown.entity(target) else {
                continue;
            };
            let base_omitted = shown.omitted_for(target).clone();
            let final_omitted: HashSet<Hex> = base_omitted.union(added).copied().collect();
            remeshes.push(spawn_remesh(
                pool,
                *cfg,
                target,
                chunk.clone(),
                final_omitted,
            ));
            omit.insert(target, (entity, added.clone()));
        }
        handovers.active_loads.push(LoadHandover {
            child: output,
            self_omit,
            omit,
            remeshes,
            results: Vec::new(),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn promote_unloads(
    commands: &mut Commands,
    handovers: &mut Handovers,
    world: &HexWorld,
    shown: &Shown,
    cfg: &WorldConfig,
    pool: &AsyncComputeTaskPool,
) {
    while handovers.active_count() < MAX_ACTIVE_HANDOVERS {
        let Some((key, entity)) = handovers.queued_unloads.pop_front() else {
            break;
        };
        // A reload raced back in before this was promoted: `key` now names a different
        // entity than the one that was unloading, so there is nothing left to restore —
        // that fresher entity is untouched (not this handover's concern; its own handover,
        // if it needs one, plans against current state on its own). But the *stale* entity
        // this handover was queued for is this handover's only reason to exist, and
        // nothing else will ever despawn it once it is dropped here — so despawn it now,
        // rather than leaking it as a permanent extra surface.
        if shown.entity(key) != Some(entity) {
            commands.entity(entity).despawn();
            continue;
        }
        let plan = entities::plan_unload_handover(shown, key);
        if plan.is_empty() {
            // Nothing to re-mesh — just despawn, no need to wait on the pool at all. Still
            // goes through `active_unloads` with an empty remesh list so `apply_finished_
            // unloads` is the one and only place a departing chunk's entity is removed.
            handovers.active_unloads.push(UnloadHandover {
                key,
                entity,
                restore: HashMap::new(),
                remeshes: Vec::new(),
                results: Vec::new(),
            });
            continue;
        }
        let targets: HashSet<ChunkKey> = plan.keys().copied().collect();
        if !targets.is_disjoint(&handovers.active_targets()) {
            handovers.queued_unloads.push_front((key, entity));
            break;
        }
        let mut remeshes = Vec::with_capacity(plan.len());
        let mut restore: HashMap<ChunkKey, (Entity, HashSet<Hex>)> =
            HashMap::with_capacity(plan.len());
        for (&target, removed) in &plan {
            // Same reasoning as `promote_loads`: a partner's store data or `Shown` entity
            // can already be gone by the time we get here. Drop the target entirely
            // rather than applying half of it.
            let Some(chunk) = world.store().chunk(target) else {
                continue;
            };
            let Some(target_entity) = shown.entity(target) else {
                continue;
            };
            let final_omitted: HashSet<Hex> = shown
                .omitted_for(target)
                .iter()
                .filter(|cell| !removed.contains(cell))
                .copied()
                .collect();
            remeshes.push(spawn_remesh(
                pool,
                *cfg,
                target,
                chunk.clone(),
                final_omitted,
            ));
            restore.insert(target, (target_entity, removed.clone()));
        }
        handovers.active_unloads.push(UnloadHandover {
            key,
            entity,
            restore,
            remeshes,
            results: Vec::new(),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_finished_loads(
    commands: &mut Commands,
    world: &mut HexWorld,
    shown: &mut Shown,
    meshes: &mut Assets<Mesh>,
    material: Option<&crate::GroundMaterialHandle>,
    handovers: &mut Handovers,
    now: f64,
) {
    let cfg = *world.store().config();
    let pool = AsyncComputeTaskPool::get();
    let mut still_active = Vec::with_capacity(handovers.active_loads.len());
    for mut handover in handovers.active_loads.drain(..) {
        // Results are kept across frames (`results`) rather than only looked at the frame
        // they land, since a handover's several re-meshes can finish on different frames —
        // only once *every* one of them is in do we have a complete, consistent picture to
        // apply together.
        handover
            .results
            .extend(poll_remeshes(&mut handover.remeshes));
        if !handover.remeshes.is_empty() {
            // Still waiting on at least one re-mesh; nothing is shown until all of them
            // land, so just keep waiting.
            still_active.push(handover);
            continue;
        }
        // A partner target may have unloaded and *reloaded* with a fresh entity while the
        // re-meshes above were in flight — fix round 2's find (task-12b-report.md): that
        // is not the same as the target simply being gone, and must not be treated as
        // such, or the target's needed re-mesh is silently dropped, leaving it and the
        // child permanently disagreeing about who draws the shared cell. Re-plan and
        // re-spawn against live state instead, and keep waiting for that too.
        if replan_reloaded_targets(
            world,
            shown,
            &cfg,
            pool,
            &mut handover.omit,
            &mut handover.results,
            &mut handover.remeshes,
            union,
        ) {
            still_active.push(handover);
            continue;
        }
        let key = handover.child.key;
        // The plan can also have grown since it was frozen at promotion — not only by an
        // existing target's entity changing (`replan_reloaded_targets`, above), but by an
        // entirely new target becoming relevant, or `key` itself needing to omit more.
        // Nothing blocks this the way `active_targets` blocks a second handover from
        // touching the same *target*: a parent or same-level neighbour that was not yet
        // shown when this handover was promoted is not reserved by anything, so it can
        // become shown — and start needing this handover's contribution — at any later
        // point while this handover waits on unrelated work; symmetrically, `key` itself
        // is not a target of anything while it has no entity, so an already-shown
        // grandchild or guest-cell owner appearing elsewhere never races a disjointness
        // check either. Reachable whenever `key` (or one of its targets) needed a
        // handover for an unrelated reason and something else settled in during that
        // wait — plain, churn-free settles included; found by the content invariant
        // (`assert_omission_invariant`, task-12b-report.md fix round 2), not by any race
        // construction.
        if reconcile_load_plan(
            world,
            shown,
            &cfg,
            pool,
            key,
            &handover.child.chunk,
            &mut handover.self_omit,
            &mut handover.omit,
            &mut handover.results,
            &mut handover.remeshes,
        ) {
            still_active.push(handover);
            continue;
        }
        // This is the same gate `insert` has always been (just reached later, once this
        // handover's re-meshes have actually landed rather than the moment generation
        // finished): a chunk the window stopped wanting while this handover was in flight
        // is dropped here, along with every finished remesh result for its
        // parent/neighbours — none of it is valid any more since `key` is never going to
        // be shown after all.
        if !world.store_mut().insert(handover.child.chunk.clone(), now) {
            continue;
        }
        // Partner chunks (parent / same-level neighbours): only touch a target that
        // *still* has exactly the entity this handover was (last) planned against — one
        // that is genuinely gone (its `TargetStatus`, not merely "not `Unchanged`" now
        // that `replan_reloaded_targets` above has already handled the `Reloaded` case) is
        // left alone entirely: neither the mesh swap nor the `shown.omit` bookkeeping runs
        // for it. Applying the bookkeeping unconditionally here was Critical 1
        // (task-12b-report.md fix round 1): it recreated a phantom omission entry for a
        // chunk with no entity, which `spawn_jobs` would then feed straight into that
        // chunk's *next* regeneration as a stale, wrong omission — a permanent hole with
        // nothing left drawing the cell.
        for (target, (entity_snapshot, cells)) in &handover.omit {
            if target_status(shown, *target, *entity_snapshot) != TargetStatus::Unchanged {
                continue;
            }
            if let Some((_, data)) = handover.results.iter().find(|(t, _)| t == target) {
                entities::apply_remesh(commands, meshes, shown, *target, data);
            }
            for cell in cells {
                shown.omit(*target, *cell);
            }
        }
        let child_mesh = handover
            .results
            .iter()
            .find(|(target, _)| *target == key)
            .map(|(_, data)| data.clone())
            .unwrap_or_else(|| handover.child.mesh.clone());
        show_child(
            commands,
            meshes,
            material,
            shown,
            world.store().config(),
            key,
            &child_mesh,
        );
        // The child's own omissions (self_dirty), if any — always applied: `show_child`
        // just gave `key` a brand new entity, so there is no earlier incarnation of it to
        // have gone stale.
        for cell in &handover.self_omit {
            shown.omit(key, *cell);
        }
    }
    handovers.active_loads = still_active;
}

fn apply_finished_unloads(
    commands: &mut Commands,
    world: &HexWorld,
    shown: &mut Shown,
    meshes: &mut Assets<Mesh>,
    handovers: &mut Handovers,
) {
    let cfg = *world.store().config();
    let pool = AsyncComputeTaskPool::get();
    let mut still_active = Vec::with_capacity(handovers.active_unloads.len());
    for mut handover in handovers.active_unloads.drain(..) {
        handover
            .results
            .extend(poll_remeshes(&mut handover.remeshes));
        if !handover.remeshes.is_empty() {
            still_active.push(handover);
            continue;
        }
        // Same reload-vs-gone distinction as `apply_finished_loads` — see fix round 2
        // (task-12b-report.md): a target that unloaded and reloaded with a fresh entity
        // must be re-planned against its live state, not treated the same as one that is
        // simply gone, or its restore is silently dropped and the cell is left with
        // nothing drawing it — a permanent hole, the unload-side mirror of Critical 1's
        // permanent-doubled-surface.
        if replan_reloaded_targets(
            world,
            shown,
            &cfg,
            pool,
            &mut handover.restore,
            &mut handover.results,
            &mut handover.remeshes,
            difference,
        ) {
            still_active.push(handover);
            continue;
        }
        // The departing chunk itself can also have reloaded with a fresh entity while
        // this handover's re-meshes were in flight — `promote_unloads`'s entity check
        // only runs once, *before* promotion; nothing re-checks it again before this
        // point. `replan_reloaded_targets` above only guards a *target*'s identity, not
        // the departing key's own — and if the departing key raced back in, the reload is
        // already drawing every cell this handover was going to restore, so applying any
        // of them now would wrongly undo an omission the reload still needs: a permanent
        // hole (found by the content invariant, fix round 2, task-12b-report.md — this
        // was the actual bug behind the residual failures fixing the load-side "new
        // target" gap didn't touch). Skip the whole restore step in that case; only the
        // stale entity's despawn below still applies.
        let departed_cleanly = shown.entity(handover.key) == Some(handover.entity);
        if departed_cleanly {
            // Only touch a target that is genuinely still `Unchanged` (the `Reloaded`
            // case is already fully handled above by `replan_reloaded_targets`).
            for (target, (entity_snapshot, cells)) in &handover.restore {
                if target_status(shown, *target, *entity_snapshot) != TargetStatus::Unchanged {
                    continue;
                }
                if let Some((_, data)) = handover.results.iter().find(|(t, _)| t == target) {
                    entities::apply_remesh(commands, meshes, shown, *target, data);
                }
                for cell in cells {
                    shown.restore(*target, *cell);
                }
            }
            shown.remove(handover.key);
        }
        commands.entity(handover.entity).despawn();
    }
    handovers.active_unloads = still_active;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_entity(index: u32) -> Entity {
        Entity::from_raw_u32(index).expect("a small index is always valid")
    }

    /// Critical 1's exact precondition, deterministically: a handover snapshotted a
    /// target's entity at promotion time, then that target unloaded (its own,
    /// independent unload handover applied — which despawns it and calls
    /// `Shown::remove`, clearing its entry entirely) before this handover's own re-mesh
    /// landed. That must read as `Gone`, not `Unchanged` — the final apply step drops a
    /// `Gone` target rather than recreating a phantom `Shown` entry for a chunk nothing
    /// is drawing any more (see `apply_finished_loads`).
    #[test]
    fn a_target_that_unloaded_since_promotion_is_gone() {
        let n = ChunkKey::new(hexworld::Level::Ken, hexworld::Hex::new(1, 0));
        let mut shown = Shown::default();
        let e1 = fake_entity(1);
        shown.insert_entity(n, e1);

        // Snapshotted while N was still shown — this is what a handover targeting N
        // would have captured at its own promotion time.
        assert_eq!(target_status(&shown, n, e1), TargetStatus::Unchanged);

        // N's own unload handover applies: despawns e1, and `Shown::remove` clears N's
        // entry entirely — exactly `apply_finished_unloads`'s last step.
        shown.remove(n);
        assert_eq!(target_status(&shown, n, e1), TargetStatus::Gone);
    }

    /// Fix round 2's find (task-12b-report.md): a target that unloads *and reloads* with
    /// a fresh entity before the other handover's re-mesh for it lands is not the same
    /// case as one that stays gone — it must read as `Reloaded(e2)`, carrying the fresh
    /// entity forward, so the caller can re-plan and re-spawn a re-mesh against it rather
    /// than silently dropping the target's needed work (fix round 1's bug: treating
    /// `Reloaded` the same as `Gone` left a permanent doubled surface or hole, depending
    /// on which side).
    #[test]
    fn a_target_that_reloaded_with_a_fresh_entity_is_reloaded_not_gone() {
        let n = ChunkKey::new(hexworld::Level::Ken, hexworld::Hex::new(1, 0));
        let mut shown = Shown::default();
        let e1 = fake_entity(1);
        shown.insert_entity(n, e1);

        shown.remove(n);
        let e2 = fake_entity(2);
        shown.insert_entity(n, e2);

        assert_eq!(target_status(&shown, n, e1), TargetStatus::Reloaded(e2));
        // Sanity: the fresh entity is of course unchanged against its own, current status.
        assert_eq!(target_status(&shown, n, e2), TargetStatus::Unchanged);
    }
}
