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
