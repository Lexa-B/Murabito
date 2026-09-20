//! A column of voxels: a sorted list of runs of material, over the full height.
//! Gaps between runs are air, so overhangs and caves can be represented.

use crate::config::WorldConfig;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Material {
    Rock,
    Dirt,
    Grass,
}

impl Material {
    /// Placeholder colours, tuned by eye.
    pub fn colour(self) -> [f32; 3] {
        match self {
            Material::Rock => [0.45, 0.44, 0.42],
            Material::Dirt => [0.42, 0.31, 0.20],
            Material::Grass => [0.34, 0.50, 0.24],
        }
    }
}

/// A run of one material, from `bottom` to `top` in layers, both inclusive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Run {
    pub bottom: i32,
    pub top: i32,
    pub material: Material,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Column {
    /// Sorted by `bottom`, non-overlapping.
    pub runs: Vec<Run>,
}

impl Column {
    pub fn top_layer(&self) -> Option<i32> {
        self.runs.last().map(|r| r.top)
    }

    pub fn material_at(&self, layer: i32) -> Option<Material> {
        self.runs
            .iter()
            .find(|r| layer >= r.bottom && layer <= r.top)
            .map(|r| r.material)
    }

    pub fn is_solid(&self, layer: i32) -> bool {
        self.material_at(layer).is_some()
    }

    /// The height of the top of the column, in metres.
    pub fn surface_height_m(&self, cfg: &WorldConfig) -> f64 {
        self.top_layer().map_or(0.0, |l| cfg.layer_top_m(l))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorldConfig;

    fn sample() -> Column {
        Column {
            runs: vec![
                Run {
                    bottom: -64,
                    top: 10,
                    material: Material::Rock,
                },
                Run {
                    bottom: 11,
                    top: 12,
                    material: Material::Dirt,
                },
                Run {
                    bottom: 13,
                    top: 13,
                    material: Material::Grass,
                },
            ],
        }
    }

    #[test]
    fn reports_its_top_layer() {
        assert_eq!(sample().top_layer(), Some(13));
        assert_eq!(Column { runs: vec![] }.top_layer(), None);
    }

    #[test]
    fn reads_material_by_layer() {
        let c = sample();
        assert_eq!(c.material_at(-64), Some(Material::Rock));
        assert_eq!(c.material_at(10), Some(Material::Rock));
        assert_eq!(c.material_at(11), Some(Material::Dirt));
        assert_eq!(c.material_at(13), Some(Material::Grass));
        assert_eq!(c.material_at(14), None);
        assert_eq!(c.material_at(-65), None);
    }

    #[test]
    fn solidity_follows_the_runs() {
        let c = Column {
            runs: vec![
                Run {
                    bottom: 0,
                    top: 2,
                    material: Material::Rock,
                },
                Run {
                    bottom: 6,
                    top: 7,
                    material: Material::Rock,
                },
            ],
        };
        assert!(c.is_solid(2));
        assert!(!c.is_solid(3), "the gap between runs is air");
        assert!(c.is_solid(6));
    }

    #[test]
    fn surface_height_is_the_top_of_the_top_run() {
        let cfg = WorldConfig::default();
        let expected = cfg.layer_top_m(13);
        assert!((sample().surface_height_m(&cfg) - expected).abs() < 1e-12);
    }
}
