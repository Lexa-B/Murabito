//! The vertical layer: a fixed 0.5 shaku (5 sun), as in `murabito_hexcoords`.

/// Height of one layer, in shaku. `murabito_hexcoords` keeps its own copy private, so a
/// test pins this one to it: if the main project ever changes it, that test fails.
pub const LAYER_HEIGHT: f32 = 0.5;

/// The layer containing a height. Layer k spans [k·h, (k+1)·h).
pub fn layer_of_height(y: f32) -> i32 {
    (y / LAYER_HEIGHT).floor() as i32
}

pub fn layer_bottom(layer: i32) -> f32 {
    layer as f32 * LAYER_HEIGHT
}

pub fn layer_top(layer: i32) -> f32 {
    (layer + 1) as f32 * LAYER_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_hexcoords::VoxelCoord;

    #[test]
    fn the_layer_height_is_main_projects() {
        let one_up = VoxelCoord::new(0, 0, 0, 1).unwrap().to_world();
        assert_eq!(one_up.y, LAYER_HEIGHT);
    }

    #[test]
    fn heights_fall_in_the_layer_below_them() {
        assert_eq!(layer_of_height(0.0), 0);
        assert_eq!(layer_of_height(0.49), 0);
        assert_eq!(layer_of_height(0.5), 1);
        assert_eq!(layer_of_height(-0.01), -1);
        assert_eq!(layer_bottom(3), 1.5);
        assert_eq!(layer_top(3), 2.0);
    }

    #[test]
    fn layer_zero_starts_at_height_zero() {
        assert_eq!(layer_of_height(0.0), 0);
        assert_eq!(layer_bottom(0), 0.0);
        assert_eq!(layer_top(0), LAYER_HEIGHT);
    }

    #[test]
    fn layers_are_signed_and_floor_downwards() {
        assert_eq!(layer_of_height(LAYER_HEIGHT * 3.5), 3);
        assert_eq!(layer_of_height(-0.001), -1);
        assert_eq!(layer_of_height(-LAYER_HEIGHT * 2.5), -3);
    }
}
