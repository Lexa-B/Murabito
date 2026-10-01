//! The server answers /render with the page's facts, and refuses what it cannot parse.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

fn start() -> SocketAddr {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind an ephemeral port");
    let addr = server.server_addr().to_ip().expect("an ip listener");
    std::thread::spawn(move || exp06::run(server));
    addr
}

fn call(addr: SocketAddr, method: &str, path: &str, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(addr).expect("connect");
    let req = format!(
        "{method} {path} HTTP/1.0\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).expect("read");
    let text = String::from_utf8_lossy(&raw).to_string();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .expect("a status line");
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    (status, body)
}

fn facts(json: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(json).expect("json");
    v["facts"]
        .as_array()
        .expect("facts")
        .iter()
        .map(|f| {
            format!(
                "{} | {}",
                f["key"].as_str().unwrap(),
                f["value"].as_str().unwrap()
            )
        })
        .collect()
}

const DEFAULT_ROWS: [&str; 6] = [
    "sea edges | 1  (SSE)",
    "land edges | 3  (NNE, NNW, W)",
    "shore edges (crossed) | 2  (E, SSW)",
    "shore crossings | 2",
    "no more than four of either | holds",
    "world centre inside the banana | yes",
];

#[test]
fn render_at_the_defaults_prints_the_reference_facts() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", "{}");
    assert_eq!(status, 200, "{body}");
    assert_eq!(facts(&body), DEFAULT_ROWS);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["path"], "f64");
    assert_eq!(v["outline"].as_array().unwrap().len(), 481);
    assert_eq!(v["edges"].as_array().unwrap().len(), 6);
    assert_eq!(v["edges"][0]["name"], "E");
    assert_eq!(v["edges"][0]["kind"], "shore");
    assert_eq!(v["marks"].as_array().unwrap().len(), 12);
    assert_eq!(v["centreline"].as_array().unwrap().len(), 61);
    assert_eq!(v["banana"]["r"], 237.0);
    assert!((v["length"].as_f64().unwrap() - 850.3012208770891).abs() < 1e-9);
}

#[test]
fn render_takes_the_pages_keys_and_partial_bodies() {
    let addr = start();
    let (status, body) = call(
        addr,
        "POST",
        "/render",
        r#"{"theta":200,"inset":-4,"octaves5":8,"windowQRS":[-6,-8,14],"unknownKey":1}"#,
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(facts(&body)[0], "sea edges | 3  (NNE, NNW, W)");
    assert_eq!(
        facts(&body)[5],
        "world centre inside the banana | no: out to sea"
    );
}

#[test]
fn the_f32_twin_answers_and_says_so() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", r#"{"path":"f32"}"#);
    assert_eq!(status, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["path"], "f32");
    assert_eq!(facts(&body), DEFAULT_ROWS);
}

#[test]
fn a_path_that_does_not_exist_yet_falls_back_to_f64() {
    let addr = start();
    let (_, body) = call(addr, "POST", "/render", r#"{"path":"gpu"}"#);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["path"], "f64");
}

#[test]
fn a_malformed_body_is_a_400_and_the_server_keeps_serving() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", "not json");
    assert_eq!(status, 400);
    assert!(body.starts_with("bad settings:"), "{body}");
    let (status, _) = call(addr, "POST", "/render", "");
    assert_eq!(status, 400);
    let (status, _) = call(addr, "POST", "/render", "{}");
    assert_eq!(status, 200);
}

#[test]
fn anything_else_is_a_404() {
    let addr = start();
    assert_eq!(call(addr, "GET", "/nothing", "").0, 404);
    assert_eq!(call(addr, "POST", "/", "").0, 404);
}

#[test]
fn the_page_is_served_at_the_root() {
    let addr = start();
    let (status, body) = call(addr, "GET", "/", "");
    assert_eq!(status, 200, "{body}");
    assert!(
        body.contains("<title>exp-06 worldgen</title>"),
        "not the page: {}",
        &body[..body.len().min(200)]
    );
    let (status, _) = call(addr, "GET", "/exp-06.html", "");
    assert_eq!(status, 200);
}

#[test]
fn an_off_plane_window_is_a_400_and_the_server_keeps_serving() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", r#"{"windowQRS":[1,1,1]}"#);
    assert_eq!(status, 400, "{body}");
    assert!(
        body.starts_with("bad settings: windowQRS off the plane"),
        "{body}"
    );
    let (status, _) = call(
        addr,
        "POST",
        "/render",
        r#"{"windowQRS":[2147483647,1,-1]}"#,
    );
    assert_eq!(status, 400);
    let (status, body) = call(addr, "POST", "/render", r#"{"windowQRS":[-6,-8,14]}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(facts(&body), DEFAULT_ROWS);
}

fn facts2(json: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(json).expect("json");
    v["facts2"]
        .as_array()
        .expect("facts2")
        .iter()
        .map(|f| {
            format!(
                "{} | {}",
                f["key"].as_str().unwrap(),
                f["value"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
fn step_2_at_the_defaults_prints_the_reference_facts2() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", "{}");
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        facts2(&body),
        [
            "spine height here, H(s) | 2455 m ≈ 16,200 layers",
            "world centre: a across, s along | 0.67, 0.58",
            "height at the origin | 269 m",
            "edge ranges, m | E -85…4 · NNE 4…791 · NNW 791…2293 · W 646…2293 · SSW -31…646 · SSE -85…-31",
            "slope just inland of the coast here | 21.5 m per ri: 100 m of height is 4.6 ri of coast",
        ]
    );
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["section"]["points"].as_array().unwrap().len(), 301);
    assert_eq!(v["heights"]["cells"].as_array().unwrap().len(), 1296);
    assert_eq!(v["heights"]["land"], 1069);
    assert_eq!(v["heights"]["cells"][0]["q"], -24);
    assert_eq!(v["heights"]["cells"][0]["s"], 12);
    assert_eq!(v["profiles"].as_array().unwrap().len(), 6);
    assert_eq!(v["profiles"][1]["name"], "NNE");
    assert_eq!(v["profiles"][1]["heights"].as_array().unwrap().len(), 121);
    assert_eq!(v["banana"]["unit_hex"].as_array().unwrap().len(), 6);
}

#[test]
fn step_2_on_the_inner_coast_and_in_f32() {
    let addr = start();
    let (_, body) = call(addr, "POST", "/render", r#"{"theta":200,"inset":-4}"#);
    assert_eq!(facts2(&body)[3], "edge ranges, m | E -20…1181 · NNE -1206…-20 · NNW -1301…-1206 · W -1299…-146 · SSW -146…848 · SSE 848…1181");
    let (_, body) = call(addr, "POST", "/render", r#"{"path":"f32"}"#);
    assert_eq!(facts2(&body)[2], "height at the origin | 269 m");
    assert_eq!(
        facts2(&body)[0],
        "spine height here, H(s) | 2455 m ≈ 16,200 layers"
    );
}
