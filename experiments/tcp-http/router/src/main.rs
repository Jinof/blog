fn main() {
    if let Err(error) = tcp_http_lab_router::run_from_env() {
        eprintln!("lab-router: {error}");
        std::process::exit(1);
    }
}
