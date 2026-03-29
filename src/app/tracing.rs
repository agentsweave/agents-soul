use std::sync::OnceLock;

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::domain::SoulError;

const LOG_ENV: &str = "AGENTS_SOUL_LOG";
const LOG_FORMAT_ENV: &str = "AGENTS_SOUL_LOG_FORMAT";
const DEFAULT_FILTER_DIRECTIVE: &str = "info";

static TRACING_INIT: OnceLock<Result<(), SoulError>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    #[default]
    Compact,
    Pretty,
    Json,
}

pub fn init_tracing() -> Result<(), SoulError> {
    TRACING_INIT
        .get_or_init(|| {
            let filter = std::env::var(LOG_ENV).unwrap_or_else(|_| DEFAULT_FILTER_DIRECTIVE.into());
            let format = match std::env::var(LOG_FORMAT_ENV) {
                Ok(value) => parse_log_format(&value)?,
                Err(std::env::VarError::NotPresent) => LogFormat::default(),
                Err(std::env::VarError::NotUnicode(_)) => {
                    return Err(SoulError::Internal(format!(
                        "`{LOG_FORMAT_ENV}` must contain valid unicode text"
                    )));
                }
            };

            init_tracing_with(&filter, format)
        })
        .clone()
}

fn init_tracing_with(filter_directive: &str, format: LogFormat) -> Result<(), SoulError> {
    let filter = EnvFilter::try_new(filter_directive).map_err(|error| {
        SoulError::Internal(format!(
            "invalid `{LOG_ENV}` filter `{filter_directive}`: {error}"
        ))
    })?;

    let registry = tracing_subscriber::registry().with(filter);

    match format {
        LogFormat::Compact => registry
            .with(fmt::layer().compact())
            .try_init()
            .map_err(map_subscriber_init_error),
        LogFormat::Pretty => registry
            .with(fmt::layer().pretty())
            .try_init()
            .map_err(map_subscriber_init_error),
        LogFormat::Json => registry
            .with(fmt::layer().json())
            .try_init()
            .map_err(map_subscriber_init_error),
    }
}

fn parse_log_format(raw: &str) -> Result<LogFormat, SoulError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "compact" => Ok(LogFormat::Compact),
        "pretty" => Ok(LogFormat::Pretty),
        "json" => Ok(LogFormat::Json),
        value => Err(SoulError::Internal(format!(
            "invalid `{LOG_FORMAT_ENV}` value `{value}`; expected compact|pretty|json"
        ))),
    }
}

fn map_subscriber_init_error(error: impl std::fmt::Display) -> SoulError {
    SoulError::Internal(format!("failed to initialize tracing subscriber: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{LogFormat, parse_log_format};

    #[test]
    fn parse_log_format_accepts_supported_values() {
        assert_eq!(
            parse_log_format("compact").expect("compact should parse"),
            LogFormat::Compact
        );
        assert_eq!(
            parse_log_format("Pretty").expect("pretty should parse"),
            LogFormat::Pretty
        );
        assert_eq!(
            parse_log_format("JSON").expect("json should parse"),
            LogFormat::Json
        );
    }

    #[test]
    fn parse_log_format_rejects_unknown_values() {
        let error = parse_log_format("verbose").expect_err("unknown format should fail");
        let message = error.to_string();
        assert!(message.contains("invalid `AGENTS_SOUL_LOG_FORMAT` value"));
    }
}
