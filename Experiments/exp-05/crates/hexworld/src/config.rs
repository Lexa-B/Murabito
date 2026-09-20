//! World-wide settings. Everything generated is a pure function of these.

use crate::level::SUN_M;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    /// Radius in ri. 0 is the single ri (0, 0); 12 is exp-03's world.
    pub world_radius_ri: i32,
    /// Thickness of one layer, in sun. 5 sun is half a shaku.
    pub layer_thickness_sun: f64,
    /// The lowest layer any column reaches.
    pub bottom_layer: i32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        WorldConfig {
            seed: 1,
            world_radius_ri: 0,
            layer_thickness_sun: 5.0,
            bottom_layer: -64,
        }
    }
}

impl WorldConfig {
    pub fn layer_thickness_m(&self) -> f64 {
        self.layer_thickness_sun * SUN_M
    }

    /// The layer containing a height. Layer k spans [k·t, (k+1)·t).
    pub fn layer_of_height(&self, height_m: f64) -> i32 {
        (height_m / self.layer_thickness_m()).floor() as i32
    }

    pub fn layer_bottom_m(&self, layer: i32) -> f64 {
        layer as f64 * self.layer_thickness_m()
    }

    pub fn layer_top_m(&self, layer: i32) -> f64 {
        (layer + 1) as f64 * self.layer_thickness_m()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let c = WorldConfig::default();
        assert_eq!(c.world_radius_ri, 0);
        assert_eq!(c.layer_thickness_sun, 5.0);
        assert!((c.layer_thickness_m() - 5.0 / 33.0).abs() < 1e-12);
    }

    #[test]
    fn layer_zero_starts_at_height_zero() {
        let c = WorldConfig::default();
        assert_eq!(c.layer_of_height(0.0), 0);
        assert!((c.layer_bottom_m(0)).abs() < 1e-12);
        assert!((c.layer_top_m(0) - 5.0 / 33.0).abs() < 1e-12);
    }

    #[test]
    fn layers_are_signed_and_floor_downwards() {
        let c = WorldConfig::default();
        let t = c.layer_thickness_m();
        assert_eq!(c.layer_of_height(t * 3.5), 3);
        assert_eq!(c.layer_of_height(-0.001), -1);
        assert_eq!(c.layer_of_height(-t * 2.5), -3);
    }

    #[test]
    fn thickness_is_tunable() {
        let c = WorldConfig {
            layer_thickness_sun: 10.0,
            ..WorldConfig::default()
        };
        assert!((c.layer_thickness_m() - 10.0 / 33.0).abs() < 1e-12);
        assert_eq!(c.layer_of_height(10.0 / 33.0), 1);
    }
}
