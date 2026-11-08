fn main() {
    match wireterm_connections::run(std::env::args_os().skip(1)) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WireTerm: {error:#}");
            std::process::exit(1);
        }
    }
}
