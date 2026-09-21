//! The levels of the hierarchy. `World` is not a unit of length: it exists so that the
//! chunk holding every ri has a parent level like any other chunk.

pub const SHAKU_M: f64 = 10.0 / 33.0;
pub const SUN_M: f64 = 1.0 / 33.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Level {
    Shaku,
    Ken,
    Cho,
    Ri,
    World,
}

/// Fine to coarse. `World` is deliberately absent: it is a chunk level, not a cell level.
pub const CELL_LEVELS: [Level; 4] = [Level::Shaku, Level::Ken, Level::Cho, Level::Ri];

impl Level {
    /// Cells of the level below per side of this cell. `Shaku` has none, so 1.
    pub fn packing(self) -> i32 {
        match self {
            Level::Shaku => 1,
            Level::Ken => 6,
            Level::Cho => 60,
            Level::Ri => 36,
            Level::World => panic!("the world level has no packing"),
        }
    }

    /// Shaku per side of this cell.
    pub fn scale_shaku(self) -> i32 {
        match self {
            Level::Shaku => 1,
            Level::Ken => 6,
            Level::Cho => 360,
            Level::Ri => 12_960,
            Level::World => panic!("the world level has no scale"),
        }
    }

    /// Flat-to-flat width in metres.
    pub fn width_m(self) -> f64 {
        self.scale_shaku() as f64 * SHAKU_M
    }

    pub fn child(self) -> Option<Level> {
        match self {
            Level::Shaku => None,
            Level::Ken => Some(Level::Shaku),
            Level::Cho => Some(Level::Ken),
            Level::Ri => Some(Level::Cho),
            Level::World => Some(Level::Ri),
        }
    }

    pub fn parent(self) -> Option<Level> {
        match self {
            Level::Shaku => Some(Level::Ken),
            Level::Ken => Some(Level::Cho),
            Level::Cho => Some(Level::Ri),
            Level::Ri => Some(Level::World),
            Level::World => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Level::Shaku => "shaku",
            Level::Ken => "ken",
            Level::Cho => "cho",
            Level::Ri => "ri",
            Level::World => "world",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packings_and_scales_are_exact() {
        assert_eq!(Level::Shaku.packing(), 1);
        assert_eq!(Level::Ken.packing(), 6);
        assert_eq!(Level::Cho.packing(), 60);
        assert_eq!(Level::Ri.packing(), 36);
        assert_eq!(Level::Shaku.scale_shaku(), 1);
        assert_eq!(Level::Ken.scale_shaku(), 6);
        assert_eq!(Level::Cho.scale_shaku(), 360);
        assert_eq!(Level::Ri.scale_shaku(), 12_960);
    }

    #[test]
    fn widths_match_the_units() {
        assert!((Level::Shaku.width_m() - 10.0 / 33.0).abs() < 1e-12);
        assert!((Level::Ken.width_m() - 60.0 / 33.0).abs() < 1e-12);
        assert!((Level::Cho.width_m() - 3_600.0 / 33.0).abs() < 1e-9);
        assert!((Level::Ri.width_m() - 129_600.0 / 33.0).abs() < 1e-9);
    }

    #[test]
    fn levels_chain_both_ways() {
        assert_eq!(Level::Shaku.parent(), Some(Level::Ken));
        assert_eq!(Level::Ri.parent(), Some(Level::World));
        assert_eq!(Level::World.parent(), None);
        assert_eq!(Level::World.child(), Some(Level::Ri));
        assert_eq!(Level::Shaku.child(), None);
    }
}
