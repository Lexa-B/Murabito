//! Every dial on the page, with the page's defaults. Always in f64; the numeric twins convert on entry.

use crate::hex::Cube;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Degrees round the outline, anticlockwise from the outer apex.
    pub theta: f64,
    /// Ri inland from the outline point; negative is out to sea.
    pub inset: f64,
    pub seed: u32,
    /// Structure amplitude at the 36-ri octave, metres.
    pub amp: f64,
    /// Hurst exponent shared by structure and erosion.
    pub hurst: f64,
    /// How much longer features are along the arc than across it.
    pub stretch: f64,
    /// Step 7a's full arrow length, m per ri (a drawing dial the page keeps in its state).
    pub gref: f64,
    /// Erosion amplitude at the 36-ri octave, metres.
    pub erosion: f64,
    pub first_octave: u32,
    pub octaves: u32,
    pub stripes: f64,
    pub sigma_ref: f64,
    pub stacking: f64,
    pub norm_above: f64,
    pub base_strength: f64,
    pub keep_deeper: f64,
    /// The window ri, if one was picked; else the ri nearest 12 ri inland.
    pub window: Option<Cube>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theta: 12.0,
            inset: 10.0,
            seed: 1,
            amp: 250.0,
            hurst: 0.65,
            stretch: 2.0,
            gref: 150.0,
            erosion: 200.0,
            first_octave: 2,
            octaves: 8,
            stripes: 1.5,
            sigma_ref: 0.03,
            stacking: 1.0,
            norm_above: 0.3,
            base_strength: 0.0,
            keep_deeper: 100.0,
            window: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_pages() {
        let s = Settings::default();
        assert_eq!((s.theta, s.inset, s.seed), (12.0, 10.0, 1));
        assert_eq!((s.amp, s.hurst, s.stretch), (250.0, 0.65, 2.0));
        assert_eq!(
            (s.erosion, s.first_octave, s.octaves, s.stripes, s.sigma_ref),
            (200.0, 2, 8, 1.5, 0.03)
        );
        assert_eq!(
            (
                s.stacking,
                s.norm_above,
                s.base_strength,
                s.keep_deeper,
                s.gref
            ),
            (1.0, 0.3, 0.0, 100.0, 150.0)
        );
        assert_eq!(s.window, None);
    }
}
