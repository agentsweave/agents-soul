fn main() -> std::process::ExitCode {
    match agents_soul::app::runtime::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            agents_soul::app::errors::map_soul_error(&err).exit_code()
        }
    }
}
