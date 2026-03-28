use std::process::ExitCode;

fn main() -> ExitCode {
    agents_soul::cli::run(std::env::args())
}
