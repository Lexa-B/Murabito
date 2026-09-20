//! Packing B and ownership: which parent each child belongs to.
//!
//! A level-L cell (a, b) is centred on the level-(L-1) cell (N·a, N·b), with no rotation
//! between levels. A child belongs to exactly one parent: the nearest parent centre, with
//! ties going to the lexicographically greatest (a, b). All integer arithmetic, so ties are
//! exact. Ownership is the same for every parent at a level, so the owned offsets are built
//! once per level and reused.

use std::sync::OnceLock;

use crate::hex::{d2, Hex};
use crate::level::Level;

/// The parent, on the lattice scaled by `n`, that owns this child cell.
pub fn owner(cell: Hex, n: i32) -> Hex {
    let a0 = (cell.q as f64 / n as f64).round() as i32;
    let b0 = (cell.r as f64 / n as f64).round() as i32;
    let mut best = Hex::ZERO;
    let mut best_key: Option<(i64, i64, i64)> = None;
    for a in (a0 - 1)..=(a0 + 1) {
        for b in (b0 - 1)..=(b0 + 1) {
            let off = Hex::new(cell.q - n * a, cell.r - n * b);
            // Smallest d2 wins; ties go to the greatest (a, b), hence the negations.
            let key = (d2(off), -(a as i64), -(b as i64));
            if best_key.is_none_or(|k| key < k) {
                best_key = Some(key);
                best = Hex::new(a, b);
            }
        }
    }
    best
}

/// The cell at the next level up that owns this one.
pub fn parent_of(cell: Hex, level: Level) -> Hex {
    let parent = level.parent().expect("the world level has no parent");
    if parent == Level::World {
        // Every ri belongs to the single world cell.
        return Hex::ZERO;
    }
    owner(cell, parent.packing())
}

/// Walk up the hierarchy from one level to another.
pub fn up(cell: Hex, from: Level, to: Level) -> Hex {
    let mut cell = cell;
    let mut level = from;
    while level != to {
        cell = parent_of(cell, level);
        level = level.parent().expect("walked past the world level");
    }
    cell
}

/// The child cell at the centre of this cell.
pub fn centre_child(cell: Hex, level: Level) -> Hex {
    let n = level.packing();
    Hex::new(cell.q * n, cell.r * n)
}

/// The shaku at the centre of this cell.
pub fn centre_shaku(cell: Hex, level: Level) -> Hex {
    let s = level.scale_shaku();
    Hex::new(cell.q * s, cell.r * s)
}

/// A cell's offset from its parent's centre, in cells of its own level.
pub fn local(cell: Hex, level: Level) -> Hex {
    let p = parent_of(cell, level);
    let n = level.parent().expect("no parent").packing();
    Hex::new(cell.q - n * p.q, cell.r - n * p.r)
}

/// Offsets, from the centre child, of the children a cell at this level owns.
/// The same for every cell at the level, so it is built once.
pub fn owned_offsets(level: Level) -> &'static [Hex] {
    static CACHE: [OnceLock<Vec<Hex>>; 5] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let slot = &CACHE[level as usize];
    slot.get_or_init(|| {
        let n = level.packing();
        let mut out = Vec::new();
        for q in -n..=n {
            for r in -n..=n {
                let c = Hex::new(q, r);
                if owner(c, n) == Hex::ZERO {
                    out.push(c);
                }
            }
        }
        out
    })
}

/// The children this cell owns, in a fixed order.
pub fn children(cell: Hex, level: Level) -> Vec<Hex> {
    let centre = centre_child(cell, level);
    owned_offsets(level).iter().map(|o| centre + *o).collect()
}

/// Offsets, from the centre child, of the children a cell at this level *draws*: every
/// child whose centre lies in this cell's ideal hexagon, boundary included. That is the
/// cells it owns, plus the "guests" — cells sitting exactly on the hexagon's border whose
/// ownership tie went to a neighbour. A guest's own hexagon straddles the border, so
/// drawing it is what stops a half-cell gap appearing along a handover.
///
/// All integer arithmetic. Deriving this from `round_at` in f64 does not work: ties on the
/// border resolve inconsistently and the result stops being a partition.
pub fn drawn_offsets(level: Level) -> &'static [Hex] {
    static CACHE: [OnceLock<Vec<Hex>>; 5] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let slot = &CACHE[level as usize];
    slot.get_or_init(|| {
        let n = level.packing();
        let mut out = Vec::new();
        for q in -n..=n {
            for r in -n..=n {
                let c = Hex::new(q, r);
                let mine = d2(c);
                let nearest = (-1..=1)
                    .flat_map(|a| (-1..=1).map(move |b| (a, b)))
                    .map(|(a, b)| d2(Hex::new(q - n * a, r - n * b)))
                    .min()
                    .expect("nine candidates");
                if mine == nearest {
                    out.push(c);
                }
            }
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::distance;

    #[test]
    fn a_parent_owns_exactly_n_squared_children() {
        assert_eq!(owned_offsets(Level::Ken).len(), 36);
        assert_eq!(owned_offsets(Level::Cho).len(), 3_600);
        assert_eq!(owned_offsets(Level::Ri).len(), 1_296);
    }

    #[test]
    fn every_child_has_exactly_one_owner() {
        // Over a patch spanning several ken, each shaku is owned by exactly one ken,
        // and that ken lists it among its children.
        for q in -20..=20 {
            for r in -20..=20 {
                let c = Hex::new(q, r);
                let ken = owner(c, Level::Ken.packing());
                let listed = children(ken, Level::Ken);
                assert!(listed.contains(&c), "{c:?} not listed by {ken:?}");
            }
        }
    }

    #[test]
    fn the_centre_child_is_owned_by_its_parent() {
        for cell in [Hex::ZERO, Hex::new(3, -7), Hex::new(-12, 5)] {
            let centre = centre_child(cell, Level::Cho);
            assert_eq!(owner(centre, Level::Cho.packing()), cell);
        }
    }

    #[test]
    fn ownership_is_translation_invariant() {
        for c in [Hex::new(1, 2), Hex::new(-4, 3), Hex::new(31, -17)] {
            let shifted = Hex::new(c.q + 6 * 5, c.r - 6 * 2);
            let a = owner(c, 6);
            let b = owner(shifted, 6);
            assert_eq!(Hex::new(b.q - a.q, b.r - a.r), Hex::new(5, -2));
        }
    }

    #[test]
    fn ties_go_to_the_greatest_parent() {
        // A shaku exactly between two ken centres: both candidates have the same d2,
        // so the lexicographically greatest (a, b) wins.
        let n = 6;
        let mut found_tie = false;
        for q in -12..=12 {
            for r in -12..=12 {
                let c = Hex::new(q, r);
                let win = owner(c, n);
                let best = crate::hex::d2(Hex::new(c.q - n * win.q, c.r - n * win.r));
                for a in -3..=3 {
                    for b in -3..=3 {
                        let cand = Hex::new(a, b);
                        if cand == win {
                            continue;
                        }
                        let d = crate::hex::d2(Hex::new(c.q - n * a, c.r - n * b));
                        if d == best {
                            found_tie = true;
                            assert!(
                                (win.q, win.r) > (cand.q, cand.r),
                                "{c:?}: {win:?} vs {cand:?}"
                            );
                        }
                    }
                }
            }
        }
        assert!(found_tie, "the patch should contain at least one tie");
    }

    #[test]
    fn owned_children_stay_near_the_centre() {
        // Packing B: owned children lie within 2n/3 of the centre, so the reach used to
        // build the template (n) is generous enough.
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let n = level.packing();
            for off in owned_offsets(level) {
                assert!(
                    distance(*off, Hex::ZERO) <= 2 * n / 3 + 1,
                    "{level:?} {off:?}"
                );
            }
        }
    }

    #[test]
    fn up_chains_levels() {
        let shaku = Hex::new(77, -31);
        let ken = parent_of(shaku, Level::Shaku);
        let cho = parent_of(ken, Level::Ken);
        assert_eq!(up(shaku, Level::Shaku, Level::Cho), cho);
        assert_eq!(up(shaku, Level::Shaku, Level::Shaku), shaku);
    }

    #[test]
    fn local_offsets_are_small() {
        let shaku = Hex::new(77, -31);
        let off = local(shaku, Level::Shaku);
        assert!(off.q.abs() <= 6 && off.r.abs() <= 6, "{off:?}");
    }
}

#[cfg(test)]
mod drawn_tests {
    use super::*;

    #[test]
    fn drawn_counts_are_exact() {
        // 36 owned + 7 guests, 3,600 + 61, 1,296 + 37: guests are ties on the hexagon
        // border, in addition to (never instead of) the owned cells.
        assert_eq!(drawn_offsets(Level::Ken).len(), 43);
        assert_eq!(drawn_offsets(Level::Cho).len(), 3_661);
        assert_eq!(drawn_offsets(Level::Ri).len(), 1_333);
    }

    #[test]
    fn the_drawn_set_contains_every_owned_cell() {
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let drawn: Vec<Hex> = drawn_offsets(level).to_vec();
            for off in owned_offsets(level) {
                assert!(drawn.contains(off), "{level:?} {off:?} owned but not drawn");
            }
        }
    }

    #[test]
    fn every_cell_is_drawn_by_its_owner() {
        // Over a patch of child cells, each cell's owner lists it among its children,
        // and the offset from that owner's centre is in the owner's drawn set.
        // No `round_at`: everything here is integer arithmetic, so it holds everywhere,
        // not just away from a boundary.
        let level = Level::Ken;
        let n = level.packing();
        for q in -9..=9 {
            for r in -9..=9 {
                let cell = Hex::new(q, r);
                let parent = owner(cell, n);
                assert!(
                    children(parent, level).contains(&cell),
                    "{cell:?} not listed by its owner {parent:?}"
                );
                let centre = centre_child(parent, level);
                let offset = Hex::new(cell.q - centre.q, cell.r - centre.r);
                assert!(
                    drawn_offsets(level).contains(&offset),
                    "{cell:?} offset {offset:?} not drawn by its owner {parent:?}"
                );
            }
        }
    }

    #[test]
    fn guests_are_tie_cells_owned_by_a_neighbour() {
        // Every offset drawn but not owned is a genuine tie — its d2 to the centre
        // matches the minimum over all nine candidate parents — and belongs to one of
        // those neighbours, not to the centre itself.
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let n = level.packing();
            let owned = owned_offsets(level);
            for off in drawn_offsets(level) {
                if owned.contains(off) {
                    continue;
                }
                assert_ne!(
                    owner(*off, n),
                    Hex::ZERO,
                    "{level:?} {off:?} is a guest but owns itself"
                );
                let mine = d2(*off);
                let nearest = (-1..=1)
                    .flat_map(|a| (-1..=1).map(move |b| (a, b)))
                    .map(|(a, b)| d2(Hex::new(off.q - n * a, off.r - n * b)))
                    .min()
                    .expect("nine candidates");
                assert_eq!(mine, nearest, "{level:?} {off:?} is a guest but not a tie");
            }
        }
    }

    #[test]
    fn the_drawn_set_is_about_the_size_of_the_owned_set() {
        // Same area, different shape: the ideal hexagon instead of battlements.
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let drawn = drawn_offsets(level).len() as f64;
            let owned = owned_offsets(level).len() as f64;
            assert!(
                (drawn - owned).abs() / owned < 0.2,
                "{level:?}: {drawn} vs {owned}"
            );
        }
    }
}
