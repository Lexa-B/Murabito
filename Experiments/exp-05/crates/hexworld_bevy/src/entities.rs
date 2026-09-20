//! One entity per chunk on screen, and the geometry that says which cells each chunk must
//! leave out because a finer chunk, or an already-shown same-level owner, draws them
//! instead.
//!
//! The centre of this module is [`desired_omissions`]: a pure, total function from *the set
//! of chunks currently on screen* to the exact omission set one chunk must be meshed with.
//! Nothing here computes an increment, and nothing freezes a plan. A mesh is applicable
//! exactly when the desired set it was computed against still equals the desired set the
//! live world asks for — one equality, checked at the moment of applying it (see
//! `tasks::step`).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use hexworld::{mesh::MeshData, plane::cell_centre_m, ChunkKey, Hex, Level, WorldConfig};

use crate::axes::{mesh_position, to_bevy};
use crate::material::GroundMaterial;

#[derive(Component, Clone, Copy, Debug)]
pub struct ChunkView(pub ChunkKey);

/// What is on screen, and the omission set each chunk's *currently applied* mesh was built
/// with.
///
/// The invariant this whole module exists to keep, restored inside every single
/// `tasks::process_handovers` call and never observable as broken from outside one:
///
/// > for every shown `key`, `omitted_for(key) == desired_omissions(cfg, &key_set(), key)`,
/// > and the mesh on `key`'s entity is `mesh_chunk(cfg, chunk_of(key), omitted_for(key))`.
///
/// `omitted` is therefore written whole (`set_omissions`), never nudged cell by cell: an
/// incremental `omit`/`restore` pair is exactly what made a stale, frozen diff expressible.
#[derive(Resource, Default)]
pub struct Shown {
    entities: HashMap<ChunkKey, Entity>,
    omitted: HashMap<ChunkKey, HashSet<Hex>>,
}

impl Shown {
    pub fn keys(&self) -> Vec<ChunkKey> {
        self.entities.keys().copied().collect()
    }

    /// Everything on screen, as the set `desired_omissions` takes.
    pub fn key_set(&self) -> HashSet<ChunkKey> {
        self.entities.keys().copied().collect()
    }

    pub fn entity(&self, key: ChunkKey) -> Option<Entity> {
        self.entities.get(&key).copied()
    }

    pub fn omitted_for(&self, key: ChunkKey) -> &HashSet<Hex> {
        static EMPTY: OnceLock<HashSet<Hex>> = OnceLock::new();
        self.omitted
            .get(&key)
            .unwrap_or_else(|| EMPTY.get_or_init(HashSet::new))
    }

    /// Every key that currently omits at least one cell. Read-only introspection for tests:
    /// a key here with no entity is a phantom omission — bookkeeping left on a chunk nothing
    /// is drawing any more.
    pub fn keys_with_omissions(&self) -> Vec<ChunkKey> {
        self.omitted
            .iter()
            .filter(|(_, cells)| !cells.is_empty())
            .map(|(key, _)| *key)
            .collect()
    }

    /// Record the omission set `key`'s newly applied mesh was built with. Always the whole
    /// set, computed fresh by `desired_omissions` — there is no way to express "add this
    /// cell" or "drop that cell" on its own, so a half-applied handover has no shape to take.
    pub fn set_omissions(&mut self, key: ChunkKey, cells: HashSet<Hex>) {
        if cells.is_empty() {
            self.omitted.remove(&key);
        } else {
            self.omitted.insert(key, cells);
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
/// its background re-mesh (see `tasks::process_handovers`) has landed.
pub fn apply_remesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    entity: Entity,
    data: &MeshData,
) {
    let handle = meshes.add(to_bevy_mesh(data));
    commands.entity(entity).insert(Mesh3d(handle));
}

/// One entry of the guest-offset table: the direction of the same-level neighbour that owns
/// a group of guest cells, and those cells' offsets from the chunk's own centre child.
type GuestGroup = (Hex, Vec<Hex>);

/// The cells a chunk at `level` draws but does not own, grouped by the same-level
/// neighbour that *does* own them, as offsets from the chunk's own centre child.
///
/// Level-uniform, so it is built once per level and shared by every chunk at it. Two facts
/// from `hexworld` make that sound, and both are checked by
/// `the_offset_table_agrees_with_drawn_cells_and_parent_of` below rather than taken on
/// trust:
///
///   * `drawn_cells(cfg, key)` is `centre_child(key.cell, key.level) + drawn_offsets(level)`
///     filtered to the world; and
///   * ownership is translation-equivariant on the parent lattice —
///     `owner(cell + n*t, n) == owner(cell, n) + t` — because the nine candidate parents
///     `owner` searches shift with `t` while their distances do not, and the tie-break
///     `-(a, b)` shifts monotonically with them.
///
/// So the guest cells of *any* chunk at the level sit at the same offsets and are owned by
/// the same *relative* neighbours. That turns the guest half of `desired_omissions` from a
/// full scan of a chunk's drawn set (3 600+ cells and a `parent_of` each, for a cho chunk)
/// into a walk over the handful of border cells owned by the neighbours that are actually
/// on screen.
fn guest_offsets_by_owner(level: Level) -> &'static [GuestGroup] {
    static CACHE: [OnceLock<Vec<GuestGroup>>; 5] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    CACHE[level as usize].get_or_init(|| {
        let n = level.packing();
        let mut by_owner: HashMap<Hex, Vec<Hex>> = HashMap::new();
        for offset in hexworld::owner::drawn_offsets(level) {
            let owner = hexworld::owner::owner(*offset, n);
            if owner != Hex::ZERO {
                by_owner.entry(owner).or_default().push(*offset);
            }
        }
        let mut out: Vec<GuestGroup> = by_owner.into_iter().collect();
        out.sort_by_key(|(dir, _)| (dir.q, dir.r));
        out
    })
}

/// **The** function this module is built around: the cells `key` must leave out of its mesh,
/// given exactly which chunks are on screen. Pure, total, and a function of `shown` alone —
/// no entities, no history, no increment, nothing frozen.
///
/// It is the union of two things, and only these two:
///
///   * (a) the cells of `key`'s own currently-shown children — they draw that ground at
///     finer detail; and
///   * (b) the guest cells `key` draws but does not own, whose true same-level owner is
///     currently shown — the owner draws them, the guest defers.
///
/// Everything else in the handover machinery is downstream of this: a finished re-mesh for
/// `T` is applicable exactly when `desired_omissions(now, T)` still equals the set it was
/// computed against, and a chunk is shown only once every chunk whose desired set its
/// arrival changes has a mesh matching that chunk's *post*-arrival desired set.
pub fn desired_omissions(
    cfg: &WorldConfig,
    shown: &HashSet<ChunkKey>,
    key: ChunkKey,
) -> HashSet<Hex> {
    let child_level = key.child_level();
    let mut out: HashSet<Hex> = HashSet::new();

    // (a) cells of currently-shown children.
    for candidate in shown {
        if candidate.level == child_level && candidate.parent_key() == Some(key) {
            out.insert(candidate.cell);
        }
    }

    // (b) guest cells whose same-level owner is currently shown. The world chunk is the
    // only chunk at its level, so it has no same-level neighbour to defer to and no guests
    // (every ri's parent is the single world cell); it also has no `packing`, so the offset
    // table must not be asked for it.
    if key.level != Level::World {
        let centre = hexworld::owner::centre_child(key.cell, key.level);
        for (dir, offsets) in guest_offsets_by_owner(key.level) {
            let owner_key = ChunkKey::new(key.level, key.cell + *dir);
            if !shown.contains(&owner_key) {
                continue;
            }
            for offset in offsets {
                let cell = centre + *offset;
                if hexworld::cell_in_world(cell, child_level, cfg) {
                    out.insert(cell);
                }
            }
        }
    }

    out
}

/// Every chunk whose `desired_omissions` can possibly change when `key` starts or stops
/// being shown — `key` itself, its parent, and the same-level neighbours it shares guest
/// cells with.
///
/// That exactness is what makes the one-frame guarantee a short argument: term (a) of
/// `desired_omissions` mentions `key` only for the chunk whose child `key` is
/// (`key.parent_key()`), and term (b) mentions it only for a same-level chunk that draws a
/// cell `key` owns.
///
/// The guest relation is *not* symmetric, and assuming it was is a mistake the unit test
/// `affected_by_names_every_chunk_whose_desired_set_a_flip_can_change` caught: a guest cell
/// only exists where two neighbouring parents are exactly equidistant, and `owner` breaks
/// that tie for the lexicographically greatest parent. So a chunk only ever cedes guests
/// *up* the lexicographic order, and the table's directions run one way. Both directions are
/// taken here: `key + dir` are the neighbours `key` cedes guests to, `key - dir` the ones
/// that cede guests to `key`. Including both is a superset for the first group, which costs
/// one desired-set comparison that comes out equal and is dropped.
///
/// **The result is deduplicated, unconditionally.** Today `key + dir` and `key - dir` cannot
/// collide, because every direction in the table is lexicographically greater than the
/// origin — but that follows from three hops of reasoning inside `hexworld::owner` (a drawn
/// offset is a guest exactly where its distance ties with the chunk's own centre; `owner`
/// breaks ties to the lexicographically greatest parent; the chunk's own cell is always
/// among the minimisers), none of which this crate states or checks. A caller that indexes
/// per returned key — `tasks::apply_partners` takes each chunk's mesh out of a map exactly
/// once — would panic on a repeat, so the safety is made explicit here rather than left
/// resting on a property of another crate that nothing asserts.
pub fn affected_by(key: ChunkKey) -> Vec<ChunkKey> {
    let mut out = vec![key];
    out.extend(key.parent_key());
    if key.level != Level::World {
        for (dir, _) in guest_offsets_by_owner(key.level) {
            out.push(ChunkKey::new(key.level, key.cell + *dir));
            out.push(ChunkKey::new(key.level, key.cell - *dir));
        }
    }
    let mut seen: HashSet<ChunkKey> = HashSet::with_capacity(out.len());
    out.retain(|candidate| seen.insert(*candidate));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexworld::owner;

    /// The same calculation `desired_omissions` makes, written the slow, obvious way:
    /// straight off `drawn_cells` and `parent_of`, with no offset table and no level-uniform
    /// reasoning. Deliberately a second implementation — it is the thing the fast one is
    /// checked against.
    fn by_direct_scan(cfg: &WorldConfig, shown: &HashSet<ChunkKey>, key: ChunkKey) -> HashSet<Hex> {
        let child_level = key.child_level();
        let mut expected: HashSet<Hex> = HashSet::new();
        for candidate in shown {
            if candidate.level == child_level && candidate.parent_key() == Some(key) {
                expected.insert(candidate.cell);
            }
        }
        for cell in hexworld::mesh::drawn_cells(cfg, key) {
            let owner_cell = owner::parent_of(cell, child_level);
            if owner_cell == key.cell {
                continue;
            }
            if shown.contains(&ChunkKey::new(key.level, owner_cell)) {
                expected.insert(cell);
            }
        }
        expected
    }

    /// Every chunk key these tests treat as the whole world: enough of each level around the
    /// origin that a chunk's parent, children and same-level neighbours are all in it — and a
    /// second cluster a long way from the origin.
    ///
    /// The off-origin cluster is not decoration. `desired_omissions` reads its guest cells out
    /// of a level-uniform offset table, which is sound only because ownership is
    /// translation-equivariant on the parent lattice — a claim *about large shifts*. A
    /// universe confined to two cells of the origin could not tell a correct table from one
    /// that happens to be right near zero. The centres are picked to stay inside the world at
    /// their own level (a ken cell runs to roughly |cell| 2 160 within a single ri, and the
    /// ri-level centre needs the radius-3 world to be in bounds at all).
    fn universe() -> Vec<ChunkKey> {
        let mut out = vec![ChunkKey::WORLD];
        for (level, far) in [
            (Level::Ken, Hex::new(500, -300)),
            (Level::Cho, Hex::new(8, -5)),
            (Level::Ri, Hex::new(2, -1)),
        ] {
            for cell in hexworld::hex::range(Hex::ZERO, 2) {
                out.push(ChunkKey::new(level, cell));
            }
            for cell in hexworld::hex::range(far, 1) {
                out.push(ChunkKey::new(level, cell));
            }
        }
        out
    }

    /// A deterministic pseudo-random subset, so the checks below see many different shown
    /// sets without a dependency or a flaky seed.
    fn subset(universe: &[ChunkKey], seed: u64) -> HashSet<ChunkKey> {
        let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        universe
            .iter()
            .copied()
            .filter(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                !(state >> 33).is_multiple_of(3)
            })
            .collect()
    }

    /// The fast, table-driven `desired_omissions` and the slow direct scan must agree on
    /// every chunk, for many different shown sets. This is what licenses the level-uniform
    /// offset table (and its translation-equivariance argument) in the first place.
    #[test]
    fn the_offset_table_agrees_with_drawn_cells_and_parent_of() {
        let cfg = WorldConfig::default();
        let universe = universe();
        // The off-origin cluster only proves anything while it is inside the world: out
        // there, `drawn_cells` would be empty and both implementations would agree on
        // nothing. Fail loudly rather than quietly if that ever stops being true.
        let far_ken = ChunkKey::new(Level::Ken, Hex::new(500, -300));
        assert!(
            !hexworld::mesh::drawn_cells(&cfg, far_ken).is_empty(),
            "{far_ken:?} must be in the world, or the far cluster tests nothing"
        );
        for seed in 0..24 {
            let shown = subset(&universe, seed);
            for &key in &universe {
                assert_eq!(
                    desired_omissions(&cfg, &shown, key),
                    by_direct_scan(&cfg, &shown, key),
                    "seed {seed}: desired_omissions disagrees with a direct scan for {key:?}"
                );
            }
        }
    }

    /// A wider world than the default single ri, so the `cell_in_world` filter is not the
    /// only thing doing the work and cho/ri chunks have real neighbours to cede guests to.
    #[test]
    fn the_offset_table_agrees_in_a_larger_world() {
        let cfg = WorldConfig {
            world_radius_ri: 3,
            ..WorldConfig::default()
        };
        let universe = universe();
        for seed in 100..112 {
            let shown = subset(&universe, seed);
            for &key in &universe {
                assert_eq!(
                    desired_omissions(&cfg, &shown, key),
                    by_direct_scan(&cfg, &shown, key),
                    "seed {seed}: desired_omissions disagrees with a direct scan for {key:?}"
                );
            }
        }
    }

    /// `affected_by` is the whole one-frame argument in one line: showing or hiding `key`
    /// can only change the desired omission set of a chunk it names. Checked by exhaustion
    /// — flip each key in the universe in and out of a shown set and look at *every* other
    /// chunk's desired set, not just the ones `affected_by` predicted.
    #[test]
    fn affected_by_names_every_chunk_whose_desired_set_a_flip_can_change() {
        let cfg = WorldConfig {
            world_radius_ri: 3,
            ..WorldConfig::default()
        };
        let universe = universe();
        for seed in 200..208 {
            let base = subset(&universe, seed);
            for &key in &universe {
                let mut without = base.clone();
                without.remove(&key);
                let mut with = base.clone();
                with.insert(key);

                let affected: HashSet<ChunkKey> = affected_by(key).into_iter().collect();
                for &other in &universe {
                    let before = desired_omissions(&cfg, &without, other);
                    let after = desired_omissions(&cfg, &with, other);
                    if before != after {
                        assert!(
                            affected.contains(&other),
                            "seed {seed}: flipping {key:?} changed {other:?}'s desired set, \
                             but affected_by({key:?}) does not name it"
                        );
                    }
                }
            }
        }
    }

    /// Ruling 2's guest dedup, stated over `desired_omissions` alone: two same-level
    /// neighbours share tie cells, and each shared cell is omitted from exactly one of them
    /// — always the side that does not own it.
    #[test]
    fn a_guest_chunk_omits_once_its_owner_is_shown() {
        let cfg = WorldConfig::default();
        let a = ChunkKey::new(Level::Ken, Hex::ZERO);
        let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));

        // Alone, neither omits anything: omission only ever follows from being shown.
        let only_a: HashSet<ChunkKey> = [a].into_iter().collect();
        assert!(desired_omissions(&cfg, &only_a, a).is_empty());

        let both: HashSet<ChunkKey> = [a, b].into_iter().collect();
        let omit_a = desired_omissions(&cfg, &both, a);
        let omit_b = desired_omissions(&cfg, &both, b);
        assert!(
            !omit_a.is_empty() || !omit_b.is_empty(),
            "a and b should share a guest cell"
        );

        let cells_a: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, a).into_iter().collect();
        let cells_b: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, b).into_iter().collect();
        for cell in cells_a.intersection(&cells_b) {
            let a_has_it = omit_a.contains(cell);
            let b_has_it = omit_b.contains(cell);
            assert!(
                a_has_it ^ b_has_it,
                "{cell:?} must be omitted from exactly one of a, b — got a={a_has_it} b={b_has_it}"
            );
            let owner_cell = owner::parent_of(*cell, Level::Shaku);
            if a_has_it {
                assert_ne!(owner_cell, a.cell, "the owner must never omit its own cell");
            } else {
                assert_ne!(owner_cell, b.cell, "the owner must never omit its own cell");
            }
        }
    }

    /// The unload side of the same rule, and the shape defect 3 used to get wrong: the
    /// guest's desired set is a function of who is shown *now*, so the owner leaving takes
    /// the cell straight back out of it — there is no separate "restore" step to miss.
    #[test]
    fn the_owner_leaving_takes_the_cell_back_out_of_the_guests_desired_set() {
        let cfg = WorldConfig::default();
        let a = ChunkKey::new(Level::Ken, Hex::ZERO);
        let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));
        let both: HashSet<ChunkKey> = [a, b].into_iter().collect();

        let cells_a: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, a).into_iter().collect();
        let cells_b: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, b).into_iter().collect();
        let shared: Vec<Hex> = cells_a.intersection(&cells_b).copied().collect();
        assert!(!shared.is_empty());

        for cell in &shared {
            let owner_cell = owner::parent_of(*cell, Level::Shaku);
            let (owner_key, guest_key) = if owner_cell == a.cell { (a, b) } else { (b, a) };
            assert!(
                desired_omissions(&cfg, &both, guest_key).contains(cell),
                "{cell:?} should be omitted from the guest side while the owner is shown"
            );
            let without_owner: HashSet<ChunkKey> = [guest_key].into_iter().collect();
            assert!(
                !desired_omissions(&cfg, &without_owner, guest_key).contains(cell),
                "{cell:?} must come straight back once {owner_key:?} is gone"
            );
        }
    }

    /// Ruling 1, and the load-order race the old design needed a dedicated "absorb the
    /// children that are already shown" step for: a parent that arrives *after* its child
    /// must omit the child's cell. With the desired set computed from the shown set, arrival
    /// order simply does not enter into it.
    #[test]
    fn a_late_parent_omits_an_already_shown_child() {
        let cfg = WorldConfig::default();
        let ken = ChunkKey::new(Level::Ken, Hex::ZERO);
        let cho = ChunkKey::new(Level::Cho, owner::parent_of(Hex::ZERO, Level::Ken));

        let child_only: HashSet<ChunkKey> = [ken].into_iter().collect();
        assert!(desired_omissions(&cfg, &child_only, cho).contains(&ken.cell));

        let both: HashSet<ChunkKey> = [ken, cho].into_iter().collect();
        assert!(desired_omissions(&cfg, &both, cho).contains(&ken.cell));
        // ...and the child never omits its parent's cell, in either order.
        assert!(!desired_omissions(&cfg, &both, ken).contains(&cho.cell));
    }

    /// `affected_by` must name the parent and the arriving chunk itself, or a handover would
    /// never re-mesh either of them.
    #[test]
    fn affected_by_names_the_chunk_and_its_parent() {
        let ken = ChunkKey::new(Level::Ken, Hex::new(1, -1));
        // No repeats: `tasks::apply_partners` takes each named chunk's mesh out of a map
        // exactly once, so a duplicate would panic there. Checked on the `Vec`, because
        // collecting into a set — which every other assertion here does — cannot see one.
        let listed = affected_by(ken);
        let affected: HashSet<ChunkKey> = listed.iter().copied().collect();
        assert_eq!(
            listed.len(),
            affected.len(),
            "affected_by returned a duplicate: {listed:?}"
        );
        assert!(affected.contains(&ken));
        assert!(affected.contains(&ken.parent_key().expect("a ken chunk has a parent")));
        for neighbour in hexworld::hex::neighbours(ken.cell) {
            assert!(
                affected.contains(&ChunkKey::new(Level::Ken, neighbour)),
                "{neighbour:?} draws cells {ken:?} owns and must be named"
            );
        }
        // The world chunk has neither a parent nor a same-level neighbour.
        assert_eq!(affected_by(ChunkKey::WORLD), vec![ChunkKey::WORLD]);
    }

    /// `set_omissions` writes the whole set, and an empty set leaves no entry behind for a
    /// later reload to pick up — the phantom-omission failure mode, made unrepresentable.
    #[test]
    fn set_omissions_replaces_rather_than_accumulates() {
        let key = ChunkKey::new(Level::Cho, Hex::ZERO);
        let mut shown = Shown::default();
        shown.set_omissions(key, [Hex::new(1, 0), Hex::new(2, 0)].into_iter().collect());
        assert_eq!(shown.omitted_for(key).len(), 2);
        shown.set_omissions(key, [Hex::new(3, 0)].into_iter().collect());
        assert_eq!(
            *shown.omitted_for(key),
            [Hex::new(3, 0)].into_iter().collect::<HashSet<_>>()
        );
        shown.set_omissions(key, HashSet::new());
        assert!(shown.omitted_for(key).is_empty());
        assert!(shown.keys_with_omissions().is_empty());
    }
}
