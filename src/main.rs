/// Runs the CLI and exits with an error status on failure.
fn main() {
    // Convert any command error into a non-zero process exit code.
    if devinit_cli_2::cli::cli_logic::cli_logic().is_err() {
        std::process::exit(1);
    }
}
