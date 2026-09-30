//! The facts rows, step by step, as the page prints them. Keys and formats are the page's, literally.

use super::banana::{Edge, EdgeKind};
use crate::format::FactRow;

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

#[cfg(test)]
mod tests {
    use super::super::banana;
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
}
