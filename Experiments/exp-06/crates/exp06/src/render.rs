//! One render: the chosen twin computes, the result is widened to f64 for the wire.

#![allow(clippy::unnecessary_cast)] // the f64 twin's `as f64` is a no-op; the f32 twin's is not

use crate::wire::{BananaOut, EdgeOut, FactOut, MarkOut, RenderResponse};
use worldgen::settings::Settings;

macro_rules! step1_with {
    ($twin:ident, $settings:expr) => {{
        use worldgen::$twin::{banana, facts, F};
        let settings = $settings;
        let started = std::time::Instant::now();
        let o = banana::outline();
        let placed = banana::place(&o, settings.theta as F, settings.inset as F);
        let corners = banana::world_corners(placed.origin);
        let edges = banana::read_edges(&o, &corners);
        let origin_inside = banana::inside(&o, placed.origin);
        let facts = facts::step1(&edges, origin_inside);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        let wide = |p: [F; 2]| [p[0] as f64, p[1] as f64];
        RenderResponse {
            path: stringify!($twin).to_string(),
            ms,
            banana: BananaOut {
                r: banana::R_RI as f64,
                w: banana::W_RI as f64,
                a: banana::ARC_DEG as f64,
                world: banana::WORLD as f64,
            },
            outline: o.pts.iter().copied().map(wide).collect(),
            length: o.length as f64,
            here: wide(placed.here.p),
            normal: wide(placed.here.normal),
            origin: wide(placed.origin),
            tangent: wide(placed.tangent),
            corners: corners.iter().copied().map(wide).collect(),
            centreline: banana::centreline().into_iter().map(wide).collect(),
            marks: banana::marks()
                .into_iter()
                .map(|(deg, m)| MarkOut {
                    deg: deg as f64,
                    p: wide(m.p),
                    normal: wide(m.normal),
                })
                .collect(),
            edges: edges
                .iter()
                .map(|e| EdgeOut {
                    name: e.direction.name().to_string(),
                    a: wide(e.a),
                    b: wide(e.b),
                    crossings: e.crossings.iter().copied().map(wide).collect(),
                    kind: e.kind.name().to_string(),
                })
                .collect(),
            facts: facts
                .into_iter()
                .map(|f| FactOut {
                    key: f.key,
                    value: f.value,
                    bad: f.bad,
                })
                .collect(),
        }
    }};
}

pub fn render(path: &str, settings: &Settings) -> RenderResponse {
    match path {
        "f32" => step1_with!(f32, settings),
        _ => step1_with!(f64, settings),
    }
}
