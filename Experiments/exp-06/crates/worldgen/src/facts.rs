//! The facts rows, step by step, as the page prints them. Keys and formats are the page's, literally.

#![allow(clippy::unnecessary_cast)]

use super::banana::{Edge, EdgeKind};
use super::tectonics::{EdgeProfile, SectionProfile};
use super::F;
use crate::format::{js_fixed, js_locale_int, js_round, FactRow};
use crate::units::LAYER_M;

/// Step 1: the edge reading and the napkin rule.
pub fn step1(edges: &[Edge], origin_inside: bool) -> Vec<FactRow> {
    let count = |k: EdgeKind| edges.iter().filter(|e| e.kind == k).count();
    let with_names = |k: EdgeKind| {
        let n = count(k);
        if n == 0 {
            return n.to_string();
        }
        let names: Vec<&str> = edges
            .iter()
            .filter(|e| e.kind == k)
            .map(|e| e.direction.name())
            .collect();
        format!("{n}  ({})", names.join(", "))
    };
    let (sea, land) = (count(EdgeKind::Sea), count(EdgeKind::Land));
    let crossings: usize = edges.iter().map(|e| e.crossings.len()).sum();
    let rule_holds = sea <= 4 && land <= 4;
    vec![
        FactRow::new("sea edges", with_names(EdgeKind::Sea), sea > 4),
        FactRow::new("land edges", with_names(EdgeKind::Land), land > 4),
        FactRow::new("shore edges (crossed)", with_names(EdgeKind::Shore), false),
        FactRow::new(
            "shore crossings",
            crossings.to_string(),
            crossings != 2 && crossings != 0,
        ),
        FactRow::new(
            "no more than four of either",
            if rule_holds { "holds" } else { "broken" },
            !rule_holds,
        ),
        FactRow::new(
            "world centre inside the banana",
            if origin_inside {
                "yes"
            } else {
                "no: out to sea"
            },
            false,
        ),
    ]
}

/// Step 2: the spine here, the world's place in the band, the origin's height, the edge ranges, the coast slope.
pub fn step2(
    section: &SectionProfile,
    origin_height: F,
    profiles: &[EdgeProfile],
    coast_slope: F,
) -> Vec<FactRow> {
    let spine = section.spine as f64;
    let layers = js_round(spine / LAYER_M) as i64;
    let ranges: Vec<String> = profiles
        .iter()
        .map(|p| {
            format!(
                "{} {}…{}",
                p.direction.name(),
                js_fixed(p.bottom as f64, 0),
                js_fixed(p.top as f64, 0)
            )
        })
        .collect();
    let slope = coast_slope as f64;
    let ri_per_100_m = if slope > 0.0 {
        js_fixed(100.0 / slope, 1)
    } else {
        "—".to_string()
    };
    vec![
        FactRow::new(
            "spine height here, H(s)",
            format!(
                "{} m ≈ {} layers",
                js_fixed(spine, 0),
                js_locale_int(layers)
            ),
            false,
        ),
        FactRow::new(
            "world centre: a across, s along",
            format!(
                "{}, {}",
                js_fixed(section.a as f64, 2),
                js_fixed(section.s as f64, 2)
            ),
            false,
        ),
        FactRow::new(
            "height at the origin",
            format!("{} m", js_fixed(origin_height as f64, 0)),
            false,
        ),
        FactRow::new("edge ranges, m", ranges.join(" · "), false),
        FactRow::new(
            "slope just inland of the coast here",
            format!(
                "{} m per ri: 100 m of height is {} ri of coast",
                js_fixed(slope, 1),
                ri_per_100_m
            ),
            false,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::banana;
    use super::super::tectonics;
    use super::super::F;
    use super::*;
    use crate::format::FactRow;

    fn rows_at(theta: F, inset: F) -> Vec<FactRow> {
        let o = banana::outline();
        let w = banana::place(&o, theta, inset);
        let edges = banana::read_edges(&o, &banana::world_corners(w.origin));
        step1(&edges, banana::inside(&o, w.origin))
    }
    fn printed(rows: &[FactRow]) -> Vec<String> {
        rows.iter()
            .map(|r| format!("{} | {}", r.key, r.value))
            .collect()
    }

    #[test]
    fn the_default_facts_are_the_reference_readings() {
        // docs/reference-readings.md, the `-- facts` block
        let rows = rows_at(12.0, 10.0);
        assert_eq!(
            printed(&rows),
            vec![
                "sea edges | 1  (SSE)",
                "land edges | 3  (NNE, NNW, W)",
                "shore edges (crossed) | 2  (E, SSW)",
                "shore crossings | 2",
                "no more than four of either | holds",
                "world centre inside the banana | yes",
            ]
        );
        assert!(rows.iter().all(|r| !r.bad));
    }

    #[test]
    fn the_inner_coast_facts() {
        let rows = rows_at(200.0, -4.0);
        assert_eq!(
            printed(&rows),
            vec![
                "sea edges | 3  (NNE, NNW, W)",
                "land edges | 1  (SSE)",
                "shore edges (crossed) | 2  (E, SSW)",
                "shore crossings | 2",
                "no more than four of either | holds",
                "world centre inside the banana | no: out to sea",
            ]
        );
    }

    #[test]
    fn far_out_to_sea_the_rule_breaks_and_says_so() {
        let rows = rows_at(12.0, -25.0);
        assert_eq!(
            printed(&rows)[0],
            "sea edges | 6  (E, NNE, NNW, W, SSW, SSE)"
        );
        assert_eq!(printed(&rows)[1], "land edges | 0");
        assert_eq!(printed(&rows)[3], "shore crossings | 0");
        assert!(!rows[3].bad, "zero crossings is fine");
        assert_eq!(printed(&rows)[4], "no more than four of either | broken");
        assert!(rows[0].bad && rows[4].bad);
    }

    fn rows2_at(theta: F, inset: F) -> Vec<FactRow> {
        let o = banana::outline();
        let w = banana::place(&o, theta, inset);
        let t = tectonics::Tectonics::new();
        let edges = banana::read_edges(&o, &banana::world_corners(w.origin));
        let profiles: Vec<_> = edges.iter().map(|e| t.edge_profile(e)).collect();
        step2(
            &t.section_at(w.origin),
            t.height(w.origin),
            &profiles,
            t.coast_slope_inland(&o, theta),
        )
    }

    #[test]
    fn the_default_facts2_are_the_reference_readings() {
        let rows = rows2_at(12.0, 10.0);
        assert_eq!(
            printed(&rows),
            vec![
                "spine height here, H(s) | 2455 m ≈ 16,200 layers",
                "world centre: a across, s along | 0.67, 0.58",
                "height at the origin | 269 m",
                "edge ranges, m | E -85…4 · NNE 4…791 · NNW 791…2293 · W 646…2293 · SSW -31…646 · SSE -85…-31",
                "slope just inland of the coast here | 21.5 m per ri: 100 m of height is 4.6 ri of coast",
            ]
        );
        assert!(rows.iter().all(|r| !r.bad));
    }

    #[test]
    fn the_inner_coast_facts2() {
        let rows = rows2_at(200.0, -4.0);
        assert_eq!(
            printed(&rows),
            vec![
                "spine height here, H(s) | 2294 m ≈ 15,139 layers",
                "world centre: a across, s along | -1.13, 0.34",
                "height at the origin | -95 m",
                "edge ranges, m | E -20…1181 · NNE -1206…-20 · NNW -1301…-1206 · W -1299…-146 · SSW -146…848 · SSE 848…1181",
                "slope just inland of the coast here | 48.3 m per ri: 100 m of height is 2.1 ri of coast",
            ]
        );
    }
}
