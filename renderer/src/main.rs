fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match wallshaders_renderer::run_cli(&args) {
        Some(code) => std::process::exit(code),
        None => {
            eprintln!("{}", wallshaders_renderer::USAGE);
            std::process::exit(2);
        }
    }
}
