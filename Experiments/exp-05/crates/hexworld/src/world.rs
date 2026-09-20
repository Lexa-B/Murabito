//! World bounds. Nothing outside them is generated or drawn.

use crate::config::WorldConfig;
use crate::hex::{distance, Hex};
use crate::level::Level;
use crate::owner::up;

pub fn in_world(ri: Hex, cfg: &WorldConfig) -> bool {
    distance(ri, Hex::ZERO) <= cfg.world_radius_ri
}

pub fn world_ri(cfg: &WorldConfig) -> Vec<Hex> {
    crate::hex::range(Hex::ZERO, cfg.world_radius_ri)
}

/// Whether a cell at any level lies in the world, tested through the ri that owns it.
pub fn cell_in_world(cell: Hex, level: Level, cfg: &WorldConfig) -> bool {
    if level == Level::World {
        return true;
    }
    in_world(up(cell, level, Level::Ri), cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_world_is_one_ri() {
        let cfg = WorldConfig::default();
        assert_eq!(world_ri(&cfg), vec![Hex::ZERO]);
        assert!(in_world(Hex::ZERO, &cfg));
        assert!(!in_world(Hex::new(1, 0), &cfg));
    }

    #[test]
    fn radius_twelve_is_exp03s_world() {
        let cfg = WorldConfig {
            world_radius_ri: 12,
            ..WorldConfig::default()
        };
        assert_eq!(world_ri(&cfg).len(), 469);
    }

    #[test]
    fn cells_are_tested_through_their_ri() {
        let cfg = WorldConfig::default();
        assert!(cell_in_world(Hex::ZERO, Level::Shaku, &cfg));
        // A shaku far outside ri (0, 0): 3 ri east.
        let far = Hex::new(3 * Level::Ri.scale_shaku(), 0);
        assert!(!cell_in_world(far, Level::Shaku, &cfg));
    }
}
