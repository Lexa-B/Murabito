fn main() {
    let server = tiny_http::Server::http(("127.0.0.1", exp06::PORT)).unwrap_or_else(|e| {
        eprintln!("cannot listen on 127.0.0.1:{}: {e}", exp06::PORT);
        std::process::exit(1)
    });
    println!(
        "exp-06 worldgen at http://127.0.0.1:{}/  serving {}",
        exp06::PORT,
        exp06::PAGE
    );
    exp06::run(server);
}
