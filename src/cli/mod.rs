pub mod compose;
pub mod configure;
pub mod explain;
pub mod inspect;
pub mod reset;

use std::process::ExitCode;

use crate::services::SoulServices;

pub fn run<I, S>(args: I) -> ExitCode
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let _program = args.next();
    let command = args.next();
    let services = SoulServices::default();

    let result = match command.as_deref() {
        Some("compose") => compose::render(&services),
        Some("inspect") => inspect::render(),
        Some("configure") => configure::render(),
        Some("reset") => reset::render(),
        Some("explain") => explain::render(),
        _ => Ok(help_text()),
    };

    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn help_text() -> String {
    "agents-soul bootstrap crate; commands: compose, inspect, configure, reset, explain".to_string()
}
