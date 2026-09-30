#![forbid(unsafe_code)]
//! The exp-06 server: serves `viz/exp-06.html` from disk and answers `POST /render` with everything
//! the page draws and prints, computed by `worldgen`. The only crate here that knows HTTP or JSON.

pub mod render;
pub mod wire;

use std::io::Cursor;

use tiny_http::{Header, Method, Request, Response, Server};

/// The fixed port; printed on start.
pub const PORT: u16 = 8060;
/// The page, read from disk on every request so an edit to it needs no restart.
pub const PAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../viz/exp-06.html");

type Reply = Response<Cursor<Vec<u8>>>;

/// Serve until the listener closes.
pub fn run(server: Server) {
    for request in server.incoming_requests() {
        handle(request);
    }
}

/// One request: the page, a render, or 404.
pub fn handle(mut request: Request) {
    let path = request.url().split('?').next().unwrap_or("/").to_string();
    let method = request.method().clone();
    let reply = match (method, path.as_str()) {
        (Method::Get, "/") | (Method::Get, "/exp-06.html") => page(),
        (Method::Post, "/render") => {
            let mut body = String::new();
            match request.as_reader().read_to_string(&mut body) {
                Ok(_) => render_json(&body),
                Err(e) => text(400, &format!("unreadable body: {e}")),
            }
        }
        _ => text(404, "not found"),
    };
    // A client that went away is not our error.
    let _ = request.respond(reply);
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("ascii header")
}

fn text(status: u16, body: &str) -> Reply {
    Response::from_string(body)
        .with_status_code(status)
        .with_header(header("Content-Type", "text/plain; charset=utf-8"))
}

fn page() -> Reply {
    match std::fs::read(PAGE) {
        Ok(bytes) => Response::from_data(bytes)
            .with_header(header("Content-Type", "text/html; charset=utf-8")),
        Err(e) => text(500, &format!("cannot read {PAGE}: {e}")),
    }
}

fn render_json(body: &str) -> Reply {
    let request: wire::RenderRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => return text(400, &format!("bad settings: {e}")),
    };
    let out = render::render(&request);
    Response::from_string(serde_json::to_string(&out).expect("a response serialises"))
        .with_header(header("Content-Type", "application/json"))
}
