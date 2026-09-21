//! The Rust port must agree with exp-03's tested Python, case for case.
//! Regenerate with: uv run --no-project python tools/make_fixtures.py

use hexworld::{local, owned_offsets, owner, up, Hex, Level};

fn level_by_name(name: &str) -> Level {
    match name {
        "shaku" => Level::Shaku,
        "ken" => Level::Ken,
        "cho" => Level::Cho,
        "ri" => Level::Ri,
        other => panic!("unknown level {other}"),
    }
}

#[test]
fn agrees_with_exp03() {
    let text = include_str!("fixtures/exp03_cases.txt");
    let mut checked = 0usize;
    for line in text.lines() {
        let (lhs, rhs) = line.split_once(" -> ").expect(line);
        let f: Vec<&str> = lhs.split_whitespace().collect();
        let r: Vec<&str> = rhs.split_whitespace().collect();
        match f[0] {
            "owner" => {
                let n: i32 = f[1].parse().unwrap();
                let cell = Hex::new(f[2].parse().unwrap(), f[3].parse().unwrap());
                let want = Hex::new(r[0].parse().unwrap(), r[1].parse().unwrap());
                assert_eq!(owner(cell, n), want, "{line}");
            }
            "up" => {
                let from = level_by_name(f[1]);
                let cell = Hex::new(f[2].parse().unwrap(), f[3].parse().unwrap());
                let to = level_by_name(f[4]);
                let want = Hex::new(r[0].parse().unwrap(), r[1].parse().unwrap());
                assert_eq!(up(cell, from, to), want, "{line}");
            }
            "local" => {
                let level = level_by_name(f[1]);
                let cell = Hex::new(f[2].parse().unwrap(), f[3].parse().unwrap());
                let want = Hex::new(r[0].parse().unwrap(), r[1].parse().unwrap());
                assert_eq!(local(cell, level), want, "{line}");
            }
            "owned_count" => {
                let level = level_by_name(f[1]);
                let want: usize = r[0].parse().unwrap();
                assert_eq!(owned_offsets(level).len(), want, "{line}");
            }
            other => panic!("unknown case {other}"),
        }
        checked += 1;
    }
    assert!(checked > 10_000, "only {checked} cases checked");
}
