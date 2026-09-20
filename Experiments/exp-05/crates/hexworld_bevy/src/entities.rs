//! One entity per chunk on screen, and the bookkeeping that keeps a chunk's mesh from
//! drawing cells a finer or sibling chunk is already drawing.

use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use hexworld::{mesh::MeshData, plane::cell_centre_m, ChunkKey, Hex, Level, WorldConfig};

use crate::axes::{mesh_position, to_bevy};
use crate::material::GroundMaterial;

#[derive(Component, Clone, Copy, Debug)]
pub struct ChunkView(pub ChunkKey);

/// What is on screen, and which cells each chunk leaves out because a finer chunk, or an
/// already-shown sibling that owns the cell, draws them instead.
#[derive(Resource, Default)]
pub struct Shown {
    entities: HashMap<ChunkKey, Entity>,
    omitted: HashMap<ChunkKey, HashSet<Hex>>,
}

impl Shown {
    pub fn keys(&self) -> Vec<ChunkKey> {
        self.entities.keys().copied().collect()
    }

    pub fn entity(&self, key: ChunkKey) -> Option<Entity> {
        self.entities.get(&key).copied()
    }

    pub fn omitted_for(&self, key: ChunkKey) -> &HashSet<Hex> {
        static EMPTY: std::sync::OnceLock<HashSet<Hex>> = std::sync::OnceLock::new();
        self.omitted
            .get(&key)
            .unwrap_or_else(|| EMPTY.get_or_init(HashSet::new))
    }

    /// Every key that currently omits at least one cell. Read-only introspection for
    /// tests: a key here with no entity is a phantom omission — bookkeeping applied
    /// against a chunk nothing is drawing any more (Critical 1, task-12b-report.md fix
    /// round 1) — which would otherwise sit undetected until that chunk happened to
    /// reload and `spawn_jobs` fed the stale entry straight into its regeneration as a
    /// wrong, permanent hole.
    pub fn keys_with_omissions(&self) -> Vec<ChunkKey> {
        self.omitted
            .iter()
            .filter(|(_, cells)| !cells.is_empty())
            .map(|(key, _)| *key)
            .collect()
    }

    pub fn omit(&mut self, parent: ChunkKey, cell: Hex) {
        self.omitted.entry(parent).or_default().insert(cell);
    }

    pub fn restore(&mut self, parent: ChunkKey, cell: Hex) {
        if let Some(set) = self.omitted.get_mut(&parent) {
            set.remove(&cell);
        }
    }

    /// Record a newly-shown chunk's entity. The caller is responsible for spawning it.
    pub fn insert_entity(&mut self, key: ChunkKey, entity: Entity) {
        self.entities.insert(key, entity);
    }

    /// Forget a chunk entirely — its entity and everything it was omitting. The caller is
    /// responsible for despawning the returned entity.
    pub fn remove(&mut self, key: ChunkKey) -> Option<Entity> {
        self.omitted.remove(&key);
        self.entities.remove(&key)
    }
}

pub fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<GroundMaterial>>) {
    let handle = materials.add(GroundMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.95,
            ..default()
        },
        extension: crate::material::GroundExtension {
            mode: crate::material::LineMode::default().as_u32(),
            tint: 0,
        },
    });
    commands.insert_resource(crate::GroundMaterialHandle(handle));
}

/// Build a Bevy mesh from the core's data, mapping the axes once.
pub fn to_bevy_mesh(data: &MeshData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        data.positions
            .iter()
            .map(|p| mesh_position(*p))
            .collect::<Vec<[f32; 3]>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        data.normals
            .iter()
            .map(|n| mesh_position(*n))
            .collect::<Vec<[f32; 3]>>(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colours.clone());
    // Line data rides in the UVs, so no custom vertex shader is needed:
    // UV0 = (edge_w, border_level), UV1 = (cell_level, 0).
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        data.line
            .iter()
            .map(|l| [l[0], l[1]])
            .collect::<Vec<[f32; 2]>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        data.line
            .iter()
            .map(|l| [l[2], 0.0])
            .collect::<Vec<[f32; 2]>>(),
    );
    mesh.insert_indices(Indices::U32(data.indices.clone()));
    mesh
}

/// Where a chunk's mesh sits in the world: the centre of its own cell.
pub fn chunk_origin(key: ChunkKey) -> Vec3 {
    if key.level == Level::World {
        Vec3::ZERO
    } else {
        let (east, north) = cell_centre_m(key.cell, key.level);
        to_bevy(east, north, 0.0)
    }
}

pub fn spawn_chunk_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<GroundMaterial>,
    key: ChunkKey,
    data: &MeshData,
    _cfg: &WorldConfig,
) -> Entity {
    let handle = meshes.add(to_bevy_mesh(data));
    commands
        .spawn((
            Mesh3d(handle),
            MeshMaterial3d(material),
            Transform::from_translation(chunk_origin(key)),
            ChunkView(key),
        ))
        .id()
}

/// Swap a chunk's `Mesh3d` to freshly computed data — the last step of a handover, once
/// its background re-mesh (see `tasks::process_handovers`) has landed. A no-op if the
/// chunk stopped being shown while that re-mesh was in flight (it unloaded, or a reload
/// raced in and swapped its entity) — there is nothing left on screen to update.
pub fn apply_remesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    shown: &Shown,
    key: ChunkKey,
    data: &MeshData,
) {
    let Some(entity) = shown.entity(key) else {
        return;
    };
    let handle = meshes.add(to_bevy_mesh(data));
    commands.entity(entity).insert(Mesh3d(handle));
}

/// The cells `key` draws that belong to a different, already-shown chunk at the same
/// level: the guest cells (`drawn_cells` minus the owned ones) whose true owner is on
/// screen. `key` is the chunk that must omit them, never the owner — the owner keeps
/// drawing every cell it owns regardless of what its neighbours are doing.
fn guest_cells_to_omit(cfg: &WorldConfig, key: ChunkKey, shown: &Shown) -> HashSet<Hex> {
    guest_cells_to_omit_considering(cfg, key, shown, None)
}

/// Same as `guest_cells_to_omit`, but also treats `pretend_shown` as shown even though it
/// has no entity yet. Used while *planning* a chunk's handover (see `plan_load_handover`):
/// the chunk that is about to be shown must already count as an owner for this check, or a
/// neighbour that owes it a guest cell would never be told to hand the cell over, but
/// nothing may actually mutate `Shown` — real entities and mesh swaps are deferred until
/// the handover's background re-meshes land — until the arriving chunk has one.
fn guest_cells_to_omit_considering(
    cfg: &WorldConfig,
    key: ChunkKey,
    shown: &Shown,
    pretend_shown: Option<ChunkKey>,
) -> HashSet<Hex> {
    let child = key.child_level();
    hexworld::mesh::drawn_cells(cfg, key)
        .into_iter()
        .filter(|cell| {
            let owner_cell = hexworld::owner::parent_of(*cell, child);
            if owner_cell == key.cell {
                return false; // key owns this cell outright; never a guest of itself
            }
            let owner_key = ChunkKey::new(key.level, owner_cell);
            Some(owner_key) == pretend_shown || shown.entity(owner_key).is_some()
        })
        .collect()
}

/// Called after `key` becomes newly shown. Brings `key`'s own guest omissions up to date
/// (some may already be owed because their true owner was shown first), and tells any
/// already-shown same-level neighbour that was drawing one of `key`'s own cells as a
/// guest to omit it now that the true owner (`key`) is on screen. Returns every key whose
/// omission set grew, `key` included, so the caller can re-mesh each of them.
pub fn sync_guests_on_load(cfg: &WorldConfig, shown: &mut Shown, key: ChunkKey) -> Vec<ChunkKey> {
    let mut changed = Vec::new();

    let mine = guest_cells_to_omit(cfg, key, shown);
    if !mine.is_empty() {
        for cell in mine {
            shown.omit(key, cell);
        }
        changed.push(key);
    }

    if key.level != Level::World {
        for neighbour_cell in hexworld::hex::neighbours(key.cell) {
            let neighbour = ChunkKey::new(key.level, neighbour_cell);
            if shown.entity(neighbour).is_none() {
                continue;
            }
            let before = shown.omitted_for(neighbour).len();
            for cell in guest_cells_to_omit(cfg, neighbour, shown) {
                shown.omit(neighbour, cell);
            }
            if shown.omitted_for(neighbour).len() != before {
                changed.push(neighbour);
            }
        }
    }

    changed
}

/// Called just before `key` unloads. Any same-level neighbour that had been omitting one
/// of `key`'s own cells (because `key`, the true owner, was shown) must draw it again, or
/// a hole opens where `key` used to be. Returns every neighbour key that changed, so the
/// caller can re-mesh each of them before `key`'s own entity despawns.
pub fn release_guests_on_unload(shown: &mut Shown, key: ChunkKey) -> Vec<ChunkKey> {
    let mut changed = Vec::new();
    if key.level == Level::World {
        return changed;
    }
    let child = key.child_level();
    for neighbour_cell in hexworld::hex::neighbours(key.cell) {
        let neighbour = ChunkKey::new(key.level, neighbour_cell);
        if shown.entity(neighbour).is_none() {
            continue;
        }
        let to_restore: Vec<Hex> = shown
            .omitted_for(neighbour)
            .iter()
            .copied()
            .filter(|cell| hexworld::owner::parent_of(*cell, child) == key.cell)
            .collect();
        if !to_restore.is_empty() {
            for cell in to_restore {
                shown.restore(neighbour, cell);
            }
            changed.push(neighbour);
        }
    }
    changed
}

/// Called after `key` becomes newly shown. If any already-shown chunk at `key`'s own
/// child level turns out to be `key`'s child (its job finished and was shown before
/// `key`'s own job did — the store's coarsest-first load order makes this rare but not
/// impossible, since only *starting* a coarser job, not finishing it, is what frees a
/// slot for a finer one), `key` must omit that child's cell too. Returns whether `key`'s
/// own omission set grew.
pub fn absorb_already_shown_children(shown: &mut Shown, key: ChunkKey) -> bool {
    let child_level = key.child_level();
    let before = shown.omitted_for(key).len();
    for candidate in shown.keys() {
        if candidate.level == child_level && candidate.parent_key() == Some(key) {
            shown.omit(key, candidate.cell);
        }
    }
    shown.omitted_for(key).len() != before
}

// --- Handover planning: pure queries the async handover (see `tasks.rs`) uses to freeze
// what a chunk's arrival or departure will require of its neighbours, *before* doing the
// (possibly expensive) work of re-meshing each of them off the main thread. Each mirrors a
// mutating function above but returns the cells-to-add per affected chunk instead of
// committing them, so the real `shown.omit`/`shown.restore` calls can be made later, in
// the same frame the background re-mesh actually lands and the chunk's entity is
// spawned/despawned — never before, or the mesh on screen would lag the bookkeeping.

/// The cells `key` itself would newly omit for an already-shown chunk at its own child
/// level that turns out to be its child — the pure-query twin of
/// `absorb_already_shown_children`.
fn plan_absorb_already_shown_children(shown: &Shown, key: ChunkKey) -> HashSet<Hex> {
    let child_level = key.child_level();
    shown
        .keys()
        .into_iter()
        .filter(|candidate| candidate.level == child_level && candidate.parent_key() == Some(key))
        .map(|candidate| candidate.cell)
        .collect()
}

/// What `key` itself would newly omit, and what each already-shown same-level neighbour
/// would newly omit, if `key` became shown right now — the pure-query twin of
/// `sync_guests_on_load`.
///
/// `guest_cells_to_omit`/`_considering` return a chunk's *full* guest set, not an
/// increment, so every candidate here is diffed against what it already omits
/// (`shown.omitted_for`) before being added to the plan — a settled same-level neighbour
/// typically already omits every guest cell it owes, and without the diff it would come
/// back as a target (and so get a real, ~18 ms re-mesh, and occupy an `active_targets`
/// slot blocking unrelated handovers) on essentially every arrival near it, for nothing.
fn plan_guest_handover_on_load(
    cfg: &WorldConfig,
    shown: &Shown,
    key: ChunkKey,
) -> HashMap<ChunkKey, HashSet<Hex>> {
    let mut plan: HashMap<ChunkKey, HashSet<Hex>> = HashMap::new();

    // `key` has no entry in `Shown` yet, so its own omitted set is always empty here and
    // this diff is a no-op — but it is *only* safe because of that, not despite it: the
    // invariant is that `Shown::omit` is never called for a key without an entity (see
    // `Shown::insert_entity`/`omit`), which is exactly what guarantees `omitted_for(key)`
    // is empty at this point. If that guarantee were ever broken, this diff would become
    // actively wrong, not merely redundant: `tasks::promote_loads` re-meshes a not-yet-
    // shown child with `added` alone (this plan's `key` entry), never
    // `shown.omitted_for(key)` unioned in, so any cell this diff excluded because it was
    // already in a non-empty `omitted_for(key)` would be missing from the child's first
    // mesh entirely — an under-omitting, doubled-surface bug. Keep this diff and
    // `promote_loads`'s self case in sync if that invariant ever changes.
    let mine: HashSet<Hex> = guest_cells_to_omit(cfg, key, shown)
        .difference(shown.omitted_for(key))
        .copied()
        .collect();
    if !mine.is_empty() {
        plan.entry(key).or_default().extend(mine);
    }

    if key.level != Level::World {
        for neighbour_cell in hexworld::hex::neighbours(key.cell) {
            let neighbour = ChunkKey::new(key.level, neighbour_cell);
            if shown.entity(neighbour).is_none() {
                continue;
            }
            let full = guest_cells_to_omit_considering(cfg, neighbour, shown, Some(key));
            let new_cells: HashSet<Hex> = full
                .difference(shown.omitted_for(neighbour))
                .copied()
                .collect();
            if !new_cells.is_empty() {
                plan.entry(neighbour).or_default().extend(new_cells);
            }
        }
    }
    plan
}

/// The full plan for showing `key`: every already-shown chunk (its parent, any same-level
/// neighbour ceding it a guest cell) that must newly omit one of `key`'s cells, plus `key`
/// itself if it must omit cells of its own (an already-shown child, or a guest cell whose
/// true owner is already on screen). Empty when nothing but `key`'s own already-meshed job
/// output is needed — the common case, handled without any of this machinery.
pub fn plan_load_handover(
    cfg: &WorldConfig,
    shown: &Shown,
    key: ChunkKey,
) -> HashMap<ChunkKey, HashSet<Hex>> {
    let mut plan: HashMap<ChunkKey, HashSet<Hex>> = HashMap::new();

    let absorbed = plan_absorb_already_shown_children(shown, key);
    if !absorbed.is_empty() {
        plan.entry(key).or_default().extend(absorbed);
    }

    if let Some(parent) = key.parent_key() {
        if shown.entity(parent).is_some() && !shown.omitted_for(parent).contains(&key.cell) {
            plan.entry(parent).or_default().insert(key.cell);
        }
    }

    for (affected, cells) in plan_guest_handover_on_load(cfg, shown, key) {
        plan.entry(affected).or_default().extend(cells);
    }

    plan
}

/// Every same-level neighbour that would take a cell back if `key` unloaded right now —
/// the pure-query twin of `release_guests_on_unload`.
fn plan_release_guests_on_unload(shown: &Shown, key: ChunkKey) -> HashMap<ChunkKey, HashSet<Hex>> {
    let mut plan: HashMap<ChunkKey, HashSet<Hex>> = HashMap::new();
    if key.level == Level::World {
        return plan;
    }
    let child = key.child_level();
    for neighbour_cell in hexworld::hex::neighbours(key.cell) {
        let neighbour = ChunkKey::new(key.level, neighbour_cell);
        if shown.entity(neighbour).is_none() {
            continue;
        }
        let to_restore: HashSet<Hex> = shown
            .omitted_for(neighbour)
            .iter()
            .copied()
            .filter(|cell| hexworld::owner::parent_of(*cell, child) == key.cell)
            .collect();
        if !to_restore.is_empty() {
            plan.insert(neighbour, to_restore);
        }
    }
    plan
}

/// The full plan for unloading `key`: the parent (if shown) that gets `key`'s cell back,
/// plus every same-level neighbour that gets a guest cell back. Empty when `key` can just
/// be despawned with nothing else to update.
pub fn plan_unload_handover(shown: &Shown, key: ChunkKey) -> HashMap<ChunkKey, HashSet<Hex>> {
    let mut plan: HashMap<ChunkKey, HashSet<Hex>> = HashMap::new();

    if let Some(parent) = key.parent_key() {
        if shown.entity(parent).is_some() {
            plan.entry(parent).or_default().insert(key.cell);
        }
    }

    for (neighbour, cells) in plan_release_guests_on_unload(shown, key) {
        plan.entry(neighbour).or_default().extend(cells);
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexworld::owner;

    fn fake_entity(index: u32) -> Entity {
        Entity::from_raw_u32(index).expect("a small index is always valid")
    }

    #[test]
    fn a_guest_chunk_omits_once_its_owner_is_shown() {
        let cfg = WorldConfig::default();
        let a = ChunkKey::new(Level::Ken, Hex::ZERO);
        let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));
        let mut shown = Shown::default();
        shown.insert_entity(a, fake_entity(1));
        shown.insert_entity(b, fake_entity(2));

        // Sanity: a and b really do share at least one guest cell (see hexworld's own
        // `shared_cells_are_exactly_guests`).
        let guests_of_a = guest_cells_to_omit(&cfg, a, &shown);
        assert!(!guests_of_a.is_empty(), "a and b should share a guest cell");

        // Neither omits anything on its own — omission only follows from being shown.
        assert!(shown.omitted_for(a).is_empty());
        assert!(shown.omitted_for(b).is_empty());

        let changed = sync_guests_on_load(&cfg, &mut shown, b);
        // b's arrival can only ever grow a's or b's own omissions, never both drop the
        // same cell: the owner (whichever of a/b actually owns a given shared cell) is
        // never added to `changed` for that cell, because `guest_cells_to_omit` only
        // ever returns cells the chunk being checked does *not* own.
        assert!(!changed.is_empty());
        for key in &changed {
            assert!(*key == a || *key == b);
        }

        // Every shared cell ends up omitted from exactly one side.
        let cells_a: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, a).into_iter().collect();
        let cells_b: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, b).into_iter().collect();
        for cell in cells_a.intersection(&cells_b) {
            let a_has_it = shown.omitted_for(a).contains(cell);
            let b_has_it = shown.omitted_for(b).contains(cell);
            assert!(
                a_has_it ^ b_has_it,
                "{cell:?} must be omitted from exactly one of a, b — got a={a_has_it} b={b_has_it}"
            );
            // Whichever side omits it must be the non-owner.
            let owner_cell = owner::parent_of(*cell, Level::Shaku);
            if a_has_it {
                assert_ne!(owner_cell, a.cell, "the owner must never omit its own cell");
            } else {
                assert_ne!(owner_cell, b.cell, "the owner must never omit its own cell");
            }
        }
    }

    #[test]
    fn unloading_the_owner_restores_the_guest() {
        let cfg = WorldConfig::default();
        let a = ChunkKey::new(Level::Ken, Hex::ZERO);
        let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));
        let mut shown = Shown::default();
        shown.insert_entity(a, fake_entity(1));
        shown.insert_entity(b, fake_entity(2));
        sync_guests_on_load(&cfg, &mut shown, a);
        sync_guests_on_load(&cfg, &mut shown, b);

        let cells_a: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, a).into_iter().collect();
        let cells_b: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, b).into_iter().collect();
        let shared: Vec<Hex> = cells_a.intersection(&cells_b).copied().collect();
        assert!(!shared.is_empty());

        // Whichever of a/b owns each shared cell: unload it, and its guest must pick the
        // cell back up rather than leaving a hole.
        for cell in &shared {
            let owner_cell = owner::parent_of(*cell, Level::Shaku);
            let (owner_key, guest_key) = if owner_cell == a.cell { (a, b) } else { (b, a) };
            assert!(
                shown.omitted_for(guest_key).contains(cell),
                "{cell:?} should have been omitted from the guest side before unload"
            );
            let restored = release_guests_on_unload(&mut shown, owner_key);
            shown.remove(owner_key);
            assert!(restored.contains(&guest_key));
            assert!(!shown.omitted_for(guest_key).contains(cell));
            // Put it back for the next cell in the loop.
            shown.insert_entity(owner_key, fake_entity(3));
            sync_guests_on_load(&cfg, &mut shown, owner_key);
        }
    }

    #[test]
    fn a_late_parent_absorbs_an_already_shown_child() {
        let ken = ChunkKey::new(Level::Ken, Hex::ZERO);
        let cho = ChunkKey::new(Level::Cho, owner::parent_of(Hex::ZERO, Level::Ken));
        let mut shown = Shown::default();
        // The child arrives (and is shown) before its parent — the rare race this
        // function exists for.
        shown.insert_entity(ken, fake_entity(1));
        assert!(shown.omitted_for(cho).is_empty());

        shown.insert_entity(cho, fake_entity(2));
        let changed = absorb_already_shown_children(&mut shown, cho);
        assert!(changed);
        assert!(shown.omitted_for(cho).contains(&ken.cell));
    }

    #[test]
    fn plan_load_handover_matches_the_late_parent_absorb_case_without_mutating_shown() {
        let ken = ChunkKey::new(Level::Ken, Hex::ZERO);
        let cho = ChunkKey::new(Level::Cho, owner::parent_of(Hex::ZERO, Level::Ken));
        let mut shown = Shown::default();
        shown.insert_entity(ken, fake_entity(1));
        shown.insert_entity(cho, fake_entity(2));

        // The plan must call out cho needing to omit ken's cell...
        let cfg = WorldConfig::default();
        let plan = plan_load_handover(&cfg, &shown, cho);
        assert_eq!(
            plan.get(&cho).cloned().unwrap_or_default(),
            [ken.cell].into_iter().collect::<HashSet<_>>()
        );
        // ...and must not itself have touched `Shown` — the whole point of a plan is that
        // the caller applies it later, once the background re-mesh it drives has landed.
        assert!(shown.omitted_for(cho).is_empty());
    }

    #[test]
    fn plan_load_handover_matches_the_guest_dedup_case_without_mutating_shown() {
        let cfg = WorldConfig::default();
        let a = ChunkKey::new(Level::Ken, Hex::ZERO);
        let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));

        // Ground truth: what `sync_guests_on_load` actually commits when b arrives after a.
        let mut mutated = Shown::default();
        mutated.insert_entity(a, fake_entity(1));
        sync_guests_on_load(&cfg, &mut mutated, a);
        mutated.insert_entity(b, fake_entity(2));
        let changed = sync_guests_on_load(&cfg, &mut mutated, b);
        assert!(!changed.is_empty(), "a and b should share a guest cell");

        // The plan, computed *before* b has an entity (the real caller's situation), must
        // predict the same omissions without mutating anything.
        let mut planned = Shown::default();
        planned.insert_entity(a, fake_entity(1));
        sync_guests_on_load(&cfg, &mut planned, a);
        let plan = plan_load_handover(&cfg, &planned, b);

        for key in [a, b] {
            let predicted = plan.get(&key).cloned().unwrap_or_default();
            assert_eq!(
                predicted,
                mutated.omitted_for(key).clone(),
                "plan for {key:?} should match what sync_guests_on_load actually commits"
            );
        }
        // The plan must not itself have touched `planned`.
        assert!(planned.omitted_for(b).is_empty());
    }
}
