use devinit_cli_2::cli::cli_logic::cli_logic;

fn main() {
    if let Err(error) = cli_logic() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
