//! World-wide settings. Everything generated is a pure function of these.

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    /// Radius in ri. 0 is the single ri (0, 0); 12 is exp-03's world.
    pub world_radius_ri: i32,
    /// The lowest layer any column reaches.
    pub bottom_layer: i32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        WorldConfig {
            seed: 1,
            world_radius_ri: 0,
            bottom_layer: -256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let c = WorldConfig::default();
        assert_eq!(c.world_radius_ri, 0);
        assert_eq!(c.seed, 1);
        assert_eq!(c.bottom_layer, -256);
    }
}
