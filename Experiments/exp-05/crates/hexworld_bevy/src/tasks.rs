//! Generation and meshing on Bevy's background task pool, and the handover that keeps a
//! chunk's mesh from drawing ground a finer or same-level chunk is already drawing.
//!
//! Chunks are pure functions of the config and the key, so a job needs nothing from the
//! main thread and can finish in any order. A chunk's *reveal*, though, can need other
//! chunks re-meshed first (its parent omitting its cell, a same-level neighbour ceding it a
//! guest cell, or the chunk itself omitting an already-shown child), and a cho chunk's
//! re-mesh alone costs ~18 ms in release — far too much for the main thread during a burst
//! of arrivals. So the re-meshes run on `AsyncComputeTaskPool` and the arriving or departing
//! chunk is *held back* until every one of them has landed; then all of it — the mesh swaps,
//! the entity spawn or despawn, and the bookkeeping — is applied in one system call.
//!
//! # Why this is structured the way it is
//!
//! Running the work asynchronously means a chunk's state can move on between deciding what
//! to do and doing it. Three review rounds found eight defects of one single shape: *a
//! chunk's shown-ness changed while some other handover's plan was frozen, and nothing
//! looked at that edge.* The fix is not another edge case. It is to stop freezing plans.
//!
//! Everything here is expressed against [`entities::desired_omissions`], a pure total
//! function from the set of shown chunks to the omission set one chunk must be meshed with:
//!
//!   * **What a handover needs is recomputed from the live shown set every frame** — never
//!     carried forward from promotion time. Growth and shrinkage are the same code path,
//!     because neither is a diff; a neighbour that becomes shown late, or an owner that
//!     departs late, simply changes the answer the next time the question is asked.
//!   * **A finished re-mesh is applicable iff the desired set it was computed against still
//!     equals the desired set asked for now** — one equality, no entity snapshots. A chunk's
//!     geometry is `mesh_chunk(cfg, chunk, omissions)` and nothing else, so a mesh that
//!     matches the desired set is right for whatever incarnation of the chunk is on screen.
//!   * **In-flight re-meshes are bookkeeping, not an invariant.** `Handovers::remeshes` holds
//!     at most one per chunk, together with the desired set it was spawned against. Two
//!     handovers wanting the same chunk re-meshed is ordinary: the first to ask this frame
//!     drives it, the other waits and asks again next frame. Nothing can clobber anything,
//!     because nothing applies a mesh whose desired set is not the one the world currently
//!     wants.
//!
//! # The one-frame guarantee
//!
//! `entities::affected_by(key)` names every chunk whose desired set can change when `key`
//! starts or stops being shown, and is exact (see its own doc and unit test). A handover
//! commits only when every one of those chunks that will still be on screen has a mesh built
//! with its *post*-change desired set. The commit applies those meshes, writes each chunk's
//! omission set whole, and spawns or despawns `key`'s entity — all inside the same
//! `process_handovers` call, so the invariant
//!
//! > for every shown `T`: `omitted_for(T) == desired_omissions(cfg, &key_set(), T)`, and
//! > `T`'s mesh is `mesh_chunk(cfg, chunk_of(T), omitted_for(T))`
//!
//! holds before and after, and is never observable in between. Chunks outside
//! `affected_by(key)` are untouched precisely because their desired sets did not change.

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

/// Start generating the chunks the store asked for, up to the in-flight cap.
///
/// Each job meshes its chunk with **nothing omitted**. That is not an assumption about what
/// the chunk will need — it is the definition the handover then works against: the job's
/// mesh is exactly `mesh_chunk(cfg, chunk, {})`, so the arriving chunk needs a re-mesh
/// precisely when its desired omission set is non-empty, and needs none when it is empty.
pub fn spawn_jobs(commands: &mut Commands, world: &mut HexWorld, to_load: Vec<ChunkKey>) {
    let cap = world.store().settings().max_in_flight;
    let running = world.store().in_flight_count();
    let pool = AsyncComputeTaskPool::get();
    for key in to_load.into_iter().take(cap.saturating_sub(running)) {
        let cfg: WorldConfig = *world.store().config();
        world.store_mut().begin_load(key);
        let task = pool.spawn(async move {
            let chunk = hexworld::chunk::generate(&cfg, key);
            let mesh = hexworld::mesh::mesh_chunk(&cfg, &chunk, &HashSet::new());
            JobOutput { key, chunk, mesh }
        });
        commands.spawn(ChunkJob(task));
    }
}

/// One chunk's re-mesh, running on the pool, and the omission set it was spawned against.
struct InFlight {
    desired: HashSet<Hex>,
    task: Task<MeshData>,
}

/// A finished re-mesh, and the omission set it was computed against.
///
/// `data` is deliberately private and there is deliberately **no** accessor that hands it
/// over without being told which omission set the caller is about to record. "Apply whatever
/// landed" is the exact shape of the defect this whole design exists to make
/// unrepresentable, and it is not enough for the apply step to merely *have* checked: the
/// only reason a mismatch is currently unreachable for a partner chunk is a scheduling
/// argument (claims are exclusive and ordered) that this module's own documentation
/// disclaims as correctness-relevant. For an *arriving* chunk it is reachable outright —
/// nothing claims a chunk that is not shown yet, so another handover can show a child of it,
/// or a same-level owner of one of its guest cells, while its own re-mesh is in flight.
/// A `debug_assert` would sit on the same line and inherit the same unreachability; a type
/// that cannot be opened without the key cannot.
struct Ready {
    desired: HashSet<Hex>,
    data: MeshData,
}

impl Ready {
    /// Whether this mesh is the mesh for exactly `want`. Probing is harmless — it cannot
    /// produce a wrong mesh, only a redundant re-spawn — so `step` uses it to decide whether
    /// it still has to wait.
    fn is_for(&self, want: &HashSet<Hex>) -> bool {
        self.desired == *want
    }

    /// The one and only way to get at the mesh: hand over the omission set about to be
    /// recorded alongside it. A mesh computed against anything else is consumed and dropped
    /// — it is a mesh for a world that has moved on, and the caller is about to spawn its
    /// replacement, so there is nothing to hand back.
    fn take_for(self, want: &HashSet<Hex>) -> Option<MeshData> {
        self.is_for(want).then_some(self.data)
    }
}

/// Take `key`'s finished re-mesh if — and only if — it is the mesh for exactly `want`.
/// Anything else is dropped rather than returned: it is a mesh for a world that has moved
/// on, and `step` is about to spawn its replacement.
fn take_ready_for(
    ready: &mut HashMap<ChunkKey, Ready>,
    key: ChunkKey,
    want: &HashSet<Hex>,
) -> Option<MeshData> {
    ready.remove(&key)?.take_for(want)
}

/// Start a chunk's re-mesh on the pool.
///
/// `chunk` is `None` when the store no longer holds the chunk's data. That happens on a
/// reachable path: `ChunkStore::update` drops a chunk's data the moment it decides to unload
/// it, while the entity lingers until that chunk's own handover applies. Generation is a
/// pure function of the config and the key, so the task simply regenerates it — which is
/// what removes the old "this target had to be dropped from the plan" case, and with it a
/// chunk left drawing over its neighbour until something else happened to fix it.
fn spawn_remesh(
    pool: &AsyncComputeTaskPool,
    cfg: WorldConfig,
    key: ChunkKey,
    chunk: Option<Chunk>,
    desired: HashSet<Hex>,
) -> InFlight {
    let omitted = desired.clone();
    let task = pool.spawn(async move {
        let chunk = chunk.unwrap_or_else(|| hexworld::chunk::generate(&cfg, key));
        hexworld::mesh::mesh_chunk(&cfg, &chunk, &omitted)
    });
    InFlight { desired, task }
}

/// One chunk's arrival or departure, waiting for the world to be able to take it.
///
/// Note what is *not* here: no plan, no list of targets, no entity snapshot of anyone else.
/// Everything a handover needs is recomputed from the live `Shown` every time it is looked
/// at, so there is nothing to go stale.
enum Handover {
    /// A generated chunk that is not on screen yet.
    Load(JobOutput),
    /// A shown chunk the store wants gone, and the exact entity that was on screen when the
    /// store said so — the one thing that genuinely identifies *which* incarnation departs.
    Unload { key: ChunkKey, entity: Entity },
}

impl Handover {
    fn key(&self) -> ChunkKey {
        match self {
            Handover::Load(output) => output.key,
            Handover::Unload { key, .. } => *key,
        }
    }
}

/// How many handovers may be waiting on the pool at once. Bounds how much of a burst lands
/// on the pool together; anything beyond the cap waits its turn, which — per the one-frame
/// rule — is invisible: a chunk that is not shown yet is exactly as correct as one that has
/// not loaded yet, and a chunk mid-unload just keeps drawing what it was already drawing.
///
/// A handover that needs no re-mesh at all does not occupy a slot: it commits the moment it
/// is looked at, so the common case (a chunk arriving where nothing else is drawn) is not
/// rate-limited by this number.
///
/// **A behaviour change from the pre-12c design, recorded deliberately.** There, a chunk
/// whose arrival changed nothing was shown unconditionally, inside `collect_finished_jobs`,
/// without consulting this cap at all. Now it goes through the same queue as everything
/// else: it still commits in the frame it is looked at and still costs no slot, but if all
/// `MAX_ACTIVE_HANDOVERS` slots are occupied *and* something ahead of it in the queue is
/// blocked, it is not looked at that frame — and it holds a `ChunkStore` in-flight slot
/// while it waits, where the old fast path would have released it immediately. The trade is
/// deliberate: one commit path instead of two that can disagree. Frame-time measurement
/// after the change was neutral, and the queue drains at the same rate, but this is the one
/// place the refactor is not purely a simplification.
const MAX_ACTIVE_HANDOVERS: usize = 3;

/// Chunk arrivals and departures that have not reached the screen yet, plus the re-meshes
/// they are waiting on.
#[derive(Resource, Default)]
pub struct Handovers {
    /// Waiting for a slot, in the order they arrived.
    queued: VecDeque<Handover>,
    /// Holding re-meshes on the pool. Oldest first: the first handover to ask for a chunk in
    /// a given frame drives that chunk's re-mesh, which is what keeps this from thrashing.
    active: Vec<Handover>,
    /// The dirty set: at most one re-mesh per chunk, with the desired set it was spawned
    /// against. Bookkeeping — no code path is required to stay out of anyone else's way for
    /// correctness, only for liveness.
    remeshes: HashMap<ChunkKey, InFlight>,
    /// Re-meshes that have landed but whose handover is still waiting on others, so they can
    /// cross frame boundaries. Kept with the desired set each was computed against, which is
    /// the entire applicability test.
    ready: HashMap<ChunkKey, Ready>,
}

impl Handovers {
    /// Queue a chunk whose departure needs a handover (`key` currently has an entity).
    pub fn queue_unload(&mut self, key: ChunkKey, entity: Entity) {
        self.queued.push_back(Handover::Unload { key, entity });
    }

    fn queue_load(&mut self, output: JobOutput) {
        self.queued.push_back(Handover::Load(output));
    }

    /// Whether `key` is a chunk whose generation job has finished but which is still being
    /// held back before being shown. Read-only introspection for tests: the one-frame rule is
    /// exactly "a key for which this is true must never also have an entity in `Shown`".
    pub fn is_pending_load(&self, key: ChunkKey) -> bool {
        self.pending(key, |h| matches!(h, Handover::Load(_)))
    }

    /// Every chunk currently held back by a load handover — see `is_pending_load`.
    pub fn pending_load_keys(&self) -> Vec<ChunkKey> {
        self.pending_keys(|h| matches!(h, Handover::Load(_)))
    }

    /// Every chunk currently held back by an unload handover, keyed by the *departing* chunk
    /// rather than by whatever it makes other chunks re-mesh. Read-only introspection for
    /// tests, which use it to know when a real backlog has built up.
    pub fn pending_unload_keys(&self) -> Vec<ChunkKey> {
        self.pending_keys(|h| matches!(h, Handover::Unload { .. }))
    }

    /// Every chunk with a re-mesh running on the pool right now. Read-only introspection for
    /// tests, in the same family as `pending_load_keys`.
    ///
    /// Intersected with `pending_load_keys()` it names exactly the chunks whose *own* re-mesh
    /// is in flight — a chunk that is not shown yet can be in here for no other reason, since
    /// nothing else has a reason to re-mesh it. That is the one window in which a chunk's
    /// desired set can drift out from under a re-mesh already computed against it, and
    /// `tests/handover.rs::a_pending_chunks_own_desired_set_can_drift_and_the_mesh_still_matches_it`
    /// needs to hit it deliberately rather than hope: without this, "pending" alone cannot
    /// tell a handover that is waiting on the pool from one still sitting in the queue, and
    /// the test passes for the wrong reason.
    pub fn remeshes_in_flight(&self) -> Vec<ChunkKey> {
        self.remeshes.keys().copied().collect()
    }

    /// Whether nothing is queued or waiting. A "settled" helper that only checks the store's
    /// own bookkeeping can read settled while a handover is still queued or mid-re-mesh: the
    /// store calls an unload gone from `loaded` well before its handover applies.
    pub fn is_idle(&self) -> bool {
        self.queued.is_empty() && self.active.is_empty()
    }

    fn pending(&self, key: ChunkKey, which: impl Fn(&Handover) -> bool) -> bool {
        self.queued
            .iter()
            .chain(self.active.iter())
            .any(|h| h.key() == key && which(h))
    }

    fn pending_keys(&self, which: impl Fn(&Handover) -> bool) -> Vec<ChunkKey> {
        self.queued
            .iter()
            .chain(self.active.iter())
            .filter(|h| which(h))
            .map(|h| h.key())
            .collect()
    }
}

/// Take finished generation jobs and queue each one. Nothing is committed to the store or to
/// `Shown` here: `process_handovers`, chained straight after this, is the single place a
/// chunk reaches (or leaves) the screen, so there is one commit path rather than a fast one
/// and a slow one that can disagree.
pub fn collect_finished_jobs(
    mut commands: Commands,
    mut handovers: ResMut<Handovers>,
    mut jobs: Query<(Entity, &mut ChunkJob)>,
) {
    for (entity, mut job) in &mut jobs {
        let Some(output) = block_on(future::poll_once(&mut job.0)) else {
            continue;
        };
        commands.entity(entity).despawn();
        handovers.queue_load(output);
    }
}

/// `step` accepts a finished re-mesh only after checking it against the same omission set
/// the commit then records, so reaching this means the two disagreed between the check and
/// the commit — impossible without a mutation of `Shown` in between, which only a commit
/// does, and a commit does not run in the middle of another.
const STALE_MESH: &str =
    "step checked this mesh against the same omission set the commit is recording";

/// What a single look at a handover concluded.
enum Outcome {
    /// Applied in full, this frame.
    Committed,
    /// No longer anything to do (the window moved on, or a newer incarnation replaced it).
    Abandoned,
    /// Re-meshes are under way for it; ask again next frame.
    Waiting(Handover),
    /// Another handover already drove one of the chunks this one needs, this frame. Nothing
    /// was started for it; ask again next frame, when it may be the one at the front.
    Blocked(Handover),
}

/// Drive every handover: collect landed re-meshes, give the waiting ones another look, then
/// promote as many queued ones as there is room for.
pub fn process_handovers(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    mut shown: ResMut<Shown>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::GroundMaterialHandle>>,
    mut handovers: ResMut<Handovers>,
) {
    // `setup_material` is a `Startup` system, so this is always present by the first
    // `Update`; the `Option` is only Bevy's way of saying the resource is inserted by a
    // command. Nothing is committed without it, rather than half-committed.
    let Some(material) = material else {
        return;
    };
    let now = time.elapsed_secs_f64();
    let cfg = *world.store().config();
    let pool = AsyncComputeTaskPool::get();
    let handovers = &mut *handovers;

    // Landed re-meshes move to `ready`, carrying the desired set they were computed against.
    let mut landed: Vec<(ChunkKey, MeshData)> = Vec::new();
    for (key, in_flight) in handovers.remeshes.iter_mut() {
        if let Some(data) = block_on(future::poll_once(&mut in_flight.task)) {
            landed.push((*key, data));
        }
    }
    for (key, data) in landed {
        let in_flight = handovers
            .remeshes
            .remove(&key)
            .expect("it was just polled out of this map");
        handovers.ready.insert(
            key,
            Ready {
                desired: in_flight.desired,
                data,
            },
        );
    }

    // One claim set for the whole frame: the first handover to ask for a chunk's re-mesh
    // owns it until the next frame. This is what makes "two handovers want the same chunk"
    // merely slow instead of a race — and it is liveness only, never correctness.
    let mut claimed: HashSet<ChunkKey> = HashSet::new();

    let mut still_active = Vec::with_capacity(handovers.active.len());
    for handover in std::mem::take(&mut handovers.active) {
        match step(
            &mut commands,
            &mut world,
            &mut shown,
            &mut meshes,
            &material,
            &cfg,
            pool,
            now,
            &mut handovers.remeshes,
            &mut handovers.ready,
            &mut claimed,
            handover,
        ) {
            Outcome::Committed | Outcome::Abandoned => {}
            Outcome::Waiting(handover) | Outcome::Blocked(handover) => still_active.push(handover),
        }
    }
    handovers.active = still_active;

    while handovers.active.len() < MAX_ACTIVE_HANDOVERS {
        let Some(handover) = handovers.queued.pop_front() else {
            break;
        };
        match step(
            &mut commands,
            &mut world,
            &mut shown,
            &mut meshes,
            &material,
            &cfg,
            pool,
            now,
            &mut handovers.remeshes,
            &mut handovers.ready,
            &mut claimed,
            handover,
        ) {
            // Committing or abandoning costs no slot, so a run of chunks that need nothing
            // drains in one frame, exactly as the old no-handover fast path did.
            Outcome::Committed | Outcome::Abandoned => {}
            Outcome::Waiting(handover) => handovers.active.push(handover),
            Outcome::Blocked(handover) => {
                // Anything behind it in the queue is younger, so letting a later one jump
                // ahead would not help this one land any sooner.
                handovers.queued.push_front(handover);
                break;
            }
        }
    }

    // Anything nobody asked for this frame is work for a world that no longer exists.
    handovers.remeshes.retain(|key, _| claimed.contains(key));
    handovers.ready.retain(|key, _| claimed.contains(key));
}

/// One look at one handover: work out — from the shown set as it is *right now* — every
/// chunk whose mesh must change for this arrival or departure and the exact omission set
/// each must be meshed with, make sure those re-meshes are under way, and commit the whole
/// thing the moment every one of them is in hand.
#[allow(clippy::too_many_arguments)]
fn step(
    commands: &mut Commands,
    world: &mut HexWorld,
    shown: &mut Shown,
    meshes: &mut Assets<Mesh>,
    material: &crate::GroundMaterialHandle,
    cfg: &WorldConfig,
    pool: &AsyncComputeTaskPool,
    now: f64,
    remeshes: &mut HashMap<ChunkKey, InFlight>,
    ready: &mut HashMap<ChunkKey, Ready>,
    claimed: &mut HashSet<ChunkKey>,
    handover: Handover,
) -> Outcome {
    let key = handover.key();

    match &handover {
        Handover::Load(output) => {
            if !world.store().is_requested(output.key) {
                // The window stopped wanting it while it waited. Commit-and-reject, purely
                // to release the store's in-flight slot for this key.
                if let Handover::Load(output) = handover {
                    world.store_mut().insert(output.chunk, now);
                }
                return Outcome::Abandoned;
            }
        }
        Handover::Unload { key, entity } => {
            if shown.entity(*key) != Some(*entity) {
                // `Shown`'s entity for a key only ever changes through a commit here, and
                // both of those commits despawn the entity they replace. So an unload naming
                // an entity that is no longer the current one names an entity that is already
                // despawned — there is nothing left to do, and despawning it a second time is
                // what used to log "tried to despawn a non-existent entity" on a perfectly
                // reachable path (a chunk queued for unload twice, or unloaded and reloaded
                // while its first unload was still queued).
                return Outcome::Abandoned;
            }
        }
    }

    // The shown set as it will be once this handover lands.
    let mut next = shown.key_set();
    match &handover {
        Handover::Load(_) => next.insert(key),
        Handover::Unload { .. } => next.remove(&key),
    };

    // Every chunk whose mesh must change, and what it must be meshed with. Recomputed in
    // full, every time: this is a statement about the world as it is now, not a diff carried
    // forward from when the handover was created.
    let mut required: Vec<(ChunkKey, HashSet<Hex>)> = Vec::new();
    for candidate in entities::affected_by(key) {
        if candidate == key {
            // The arriving chunk itself: its generation job meshed it with nothing omitted,
            // so it needs a re-mesh exactly when its own desired set is non-empty. A
            // departing chunk is about to lose its entity, so it needs nothing.
            if matches!(handover, Handover::Load(_)) {
                let desired = entities::desired_omissions(cfg, &next, key);
                if !desired.is_empty() {
                    required.push((key, desired));
                }
            }
            continue;
        }
        if !next.contains(&candidate) {
            continue; // not on screen once this lands: nothing of it to update
        }
        let desired = entities::desired_omissions(cfg, &next, candidate);
        if desired != *shown.omitted_for(candidate) {
            required.push((candidate, desired));
        }
    }

    if required.iter().any(|(chunk, _)| claimed.contains(chunk)) {
        return Outcome::Blocked(handover);
    }

    let mut all_ready = true;
    for (chunk_key, desired) in &required {
        claimed.insert(*chunk_key);
        if ready.get(chunk_key).is_some_and(|r| r.is_for(desired)) {
            continue;
        }
        all_ready = false;
        if remeshes
            .get(chunk_key)
            .is_some_and(|f| f.desired == *desired)
        {
            continue;
        }
        // Nothing is running for this chunk, or what is running was computed against a set
        // the world has since moved past. Either way, start the right one; the stale result
        // is dropped rather than applied, because applying it is the only thing that could
        // ever be wrong.
        ready.remove(chunk_key);
        let chunk = match &handover {
            Handover::Load(output) if *chunk_key == key => Some(output.chunk.clone()),
            _ => world.store().chunk(*chunk_key).cloned(),
        };
        remeshes.insert(
            *chunk_key,
            spawn_remesh(pool, *cfg, *chunk_key, chunk, desired.clone()),
        );
    }
    if !all_ready {
        return Outcome::Waiting(handover);
    }

    commit(
        commands, world, shown, meshes, material, cfg, now, ready, handover, required,
    )
}

/// Apply a handover in full: every changed chunk's new mesh and new omission set, plus the
/// arriving chunk's entity or the departing chunk's despawn — all in this one call, which is
/// what the one-frame guarantee is.
#[allow(clippy::too_many_arguments)]
fn commit(
    commands: &mut Commands,
    world: &mut HexWorld,
    shown: &mut Shown,
    meshes: &mut Assets<Mesh>,
    material: &crate::GroundMaterialHandle,
    cfg: &WorldConfig,
    now: f64,
    ready: &mut HashMap<ChunkKey, Ready>,
    handover: Handover,
    required: Vec<(ChunkKey, HashSet<Hex>)>,
) -> Outcome {
    let key = handover.key();

    // Take the arriving chunk's own re-mesh out of the list first: it is applied by spawning
    // an entity, not by swapping one.
    let mut own: Option<HashSet<Hex>> = None;
    let mut partners: Vec<(ChunkKey, HashSet<Hex>)> = Vec::with_capacity(required.len());
    for (chunk_key, desired) in required {
        if chunk_key == key {
            own = Some(desired);
        } else {
            partners.push((chunk_key, desired));
        }
    }

    match handover {
        Handover::Load(output) => {
            // The single gate for "the window stopped wanting this": reached here rather
            // than when generation finished, so `in_flight_count()`/`loaded_keys()` — and so
            // every settle check — mean "genuinely shown", not merely "generated".
            if !world.store_mut().insert(output.chunk, now) {
                // Nothing of this handover is valid any more, so drop the meshes it was
                // holding rather than leaving them for a world that will never ask again.
                ready.remove(&key);
                for (chunk_key, _) in &partners {
                    ready.remove(chunk_key);
                }
                return Outcome::Abandoned;
            }
            apply_partners(commands, shown, meshes, ready, partners);

            // `own` is `Some` exactly when `step` required a re-mesh of the arriving chunk
            // itself, and `take_ready_for` hands it over only if it was computed against the
            // very set recorded below. This is the reachable half of the staleness check: a
            // chunk that is not shown yet is claimed by nobody, so its desired set can drift
            // between spawning that re-mesh and getting here.
            let own_mesh = own
                .as_ref()
                .map(|desired| take_ready_for(ready, key, desired).expect(STALE_MESH));
            let mesh = own_mesh.as_ref().unwrap_or(&output.mesh);

            // A reload can race back in while this chunk's own unload is still queued, so
            // `key` may already name an older entity. Replace it here, where the new one is
            // spawned, so there is exactly one moment at which a key's entity changes and
            // the old one is always despawned at it.
            if let Some(stale) = shown.remove(key) {
                commands.entity(stale).despawn();
            }
            let entity =
                entities::spawn_chunk_entity(commands, meshes, material.0.clone(), key, mesh, cfg);
            shown.insert_entity(key, entity);
            shown.set_omissions(key, own.unwrap_or_default());
        }
        Handover::Unload { key, entity } => {
            apply_partners(commands, shown, meshes, ready, partners);
            shown.remove(key);
            commands.entity(entity).despawn();
        }
    }

    Outcome::Committed
}

/// Swap in each partner chunk's new mesh and record the omission set it was built with. The
/// two always move together — that pairing is the content invariant.
fn apply_partners(
    commands: &mut Commands,
    shown: &mut Shown,
    meshes: &mut Assets<Mesh>,
    ready: &mut HashMap<ChunkKey, Ready>,
    partners: Vec<(ChunkKey, HashSet<Hex>)>,
) {
    for (chunk_key, desired) in partners {
        // The mesh and the omission set recorded beside it come out of the same call, keyed
        // by that very set — there is no way to write "apply whatever landed" here.
        let data = take_ready_for(ready, chunk_key, &desired).expect(STALE_MESH);
        let entity = shown
            .entity(chunk_key)
            .expect("a partner is only required while it is on screen");
        entities::apply_remesh(commands, meshes, entity, &data);
        shown.set_omissions(chunk_key, desired);
    }
}
