//! The units the readings are printed in. A ri is 12,960 shaku of 10/33 m; a layer is 5 sun, half a shaku.

/// Metres in a ri, flat to flat of a ri cell.
pub const RI_M: f64 = 3927.0;
/// Metres in a layer: the page's constant, 5 sun.
pub const LAYER_M: f64 = 0.15151;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ri_is_3927_metres_and_a_layer_half_a_shaku() {
        assert_eq!(RI_M, 3927.0);
        assert_eq!(LAYER_M, 0.15151);
        assert!((RI_M / LAYER_M - 25919.08).abs() < 0.01); // about 25,919 layers to a ri
    }
}
