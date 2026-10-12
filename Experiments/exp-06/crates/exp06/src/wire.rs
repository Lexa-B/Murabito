//! What crosses the wire: the page's state in, the drawing's data out. Key names are the page's.

use serde::{Deserialize, Serialize};
use worldgen::hex::Cube;
use worldgen::settings::Settings;

/// The page's `state`, plus which compute path to run. Missing keys keep the page's defaults;
/// unknown keys are ignored.
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RenderRequest {
    pub theta: f64,
    pub inset: f64,
    pub seed: u32,
    pub amp: f64,
    pub hurst: f64,
    pub stretch: f64,
    pub gref: f64,
    pub erosion: f64,
    pub first_octave: u32,
    pub octaves5: u32,
    pub stripes: f64,
    pub sigma_ref: f64,
    pub stacking: f64,
    pub norm_above: f64,
    pub base_strength: f64,
    pub keep_deeper: f64,
    #[serde(rename = "windowQRS")]
    pub window_qrs: Option<[i32; 3]>,
    /// `"f64"` (the reference), `"f32"` (the shader's twin); anything else runs f64.
    pub path: String,
}

impl Default for RenderRequest {
    fn default() -> Self {
        let s = Settings::default();
        RenderRequest {
            theta: s.theta,
            inset: s.inset,
            seed: s.seed,
            amp: s.amp,
            hurst: s.hurst,
            stretch: s.stretch,
            gref: s.gref,
            erosion: s.erosion,
            first_octave: s.first_octave,
            octaves5: s.octaves,
            stripes: s.stripes,
            sigma_ref: s.sigma_ref,
            stacking: s.stacking,
            norm_above: s.norm_above,
            base_strength: s.base_strength,
            keep_deeper: s.keep_deeper,
            window_qrs: None,
            path: "f64".to_string(),
        }
    }
}

impl RenderRequest {
    /// The page's state as settings. A window that is not on the plane (`q + r + s ≠ 0`, or a sum that
    /// overflows) is refused here, so it can never reach `Cube::new`'s assertion inside the server.
    pub fn settings(&self) -> Result<Settings, String> {
        let window = match self.window_qrs {
            None => None,
            Some([q, r, s]) => {
                let sum = q.checked_add(r).and_then(|t| t.checked_add(s));
                if sum != Some(0) {
                    return Err(format!(
                        "windowQRS off the plane: [{q}, {r}, {s}] does not sum to 0"
                    ));
                }
                Some(Cube::new(q, r, s))
            }
        };
        Ok(Settings {
            theta: self.theta,
            inset: self.inset,
            seed: self.seed,
            amp: self.amp,
            hurst: self.hurst,
            stretch: self.stretch,
            gref: self.gref,
            erosion: self.erosion,
            first_octave: self.first_octave,
            octaves: self.octaves5,
            stripes: self.stripes,
            sigma_ref: self.sigma_ref,
            stacking: self.stacking,
            norm_above: self.norm_above,
            base_strength: self.base_strength,
            keep_deeper: self.keep_deeper,
            window,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct BananaOut {
    pub r: f64,
    pub w: f64,
    pub a: f64,
    pub world: f64,
    pub unit_hex: Vec<[f64; 2]>,
}

#[derive(Debug, Serialize)]
pub struct MarkOut {
    pub deg: f64,
    pub p: [f64; 2],
    pub normal: [f64; 2],
}

#[derive(Debug, Serialize)]
pub struct EdgeOut {
    pub name: String,
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub crossings: Vec<[f64; 2]>,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct FactOut {
    pub key: String,
    pub value: String,
    pub bad: bool,
}

#[derive(Debug, Serialize)]
pub struct SectionOut {
    pub s: f64,
    pub spine: f64,
    pub a: f64,
    pub points: Vec<[f64; 2]>,
}

#[derive(Debug, Serialize)]
pub struct RiOut {
    pub q: i32,
    pub r: i32,
    pub s: i32,
    pub c: [f64; 2],
    pub h: f64,
}

#[derive(Debug, Serialize)]
pub struct HeightsOut {
    pub cells: Vec<RiOut>,
    pub land: usize,
    pub highest: f64,
    pub deepest: f64,
    pub land_mean: f64,
}

#[derive(Debug, Serialize)]
pub struct ProfileOut {
    pub name: String,
    pub kind: String,
    pub heights: Vec<f64>,
    pub bottom: f64,
    pub top: f64,
}

/// Everything steps 1 and 2 draw and prints. Later bites append fields; nothing here is renamed.
#[derive(Debug, Serialize)]
pub struct RenderResponse {
    pub path: String,
    /// Server time for the render, milliseconds.
    pub ms: f64,
    pub banana: BananaOut,
    pub outline: Vec<[f64; 2]>,
    pub length: f64,
    pub here: [f64; 2],
    pub normal: [f64; 2],
    pub origin: [f64; 2],
    pub tangent: [f64; 2],
    pub corners: Vec<[f64; 2]>,
    pub centreline: Vec<[f64; 2]>,
    pub marks: Vec<MarkOut>,
    pub edges: Vec<EdgeOut>,
    pub facts: Vec<FactOut>,
    pub section: SectionOut,
    pub heights: HeightsOut,
    pub profiles: Vec<ProfileOut>,
    pub facts2: Vec<FactOut>,
}
