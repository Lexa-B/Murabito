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
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
