use serde::{Deserialize, Serialize};

use crate::{
    api::{
        heuristics::{UpdateHeuristicsRequest, handle_update_heuristics},
        interactions::{
            RecordInteractionRequest, handle_record_interaction, record_error_response,
            record_success_status,
        },
        reset::{ResetAdaptationRequest, handle_reset_adaptation, reset_error_response},
        traits::{UpdateTraitsRequest, handle_update_traits},
    },
    app::{
        deps::SoulDependencies,
        errors::{SoulHttpErrorBody, SoulHttpErrorResponse, map_soul_error},
    },
    domain::SoulError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Patch,
    Post,
}

impl HttpMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Patch => "PATCH",
            Self::Post => "POST",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiEndpoint {
    pub method: HttpMethod,
    pub path: &'static str,
    pub handler: &'static str,
}

pub const UPDATE_TRAITS_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Patch,
    path: "/v1/traits",
    handler: "api::traits::handle_update_traits",
};
pub const UPDATE_HEURISTICS_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Patch,
    path: "/v1/heuristics",
    handler: "api::heuristics::handle_update_heuristics",
};
pub const RECORD_INTERACTION_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Post,
    path: "/v1/interactions",
    handler: "api::interactions::handle_record_interaction",
};
pub const RESET_ADAPTATION_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Post,
    path: "/v1/reset",
    handler: "api::reset::handle_reset_adaptation",
};

const WRITE_ENDPOINTS: [ApiEndpoint; 4] = [
    UPDATE_TRAITS_ROUTE,
    UPDATE_HEURISTICS_ROUTE,
    RECORD_INTERACTION_ROUTE,
    RESET_ADAPTATION_ROUTE,
];

pub fn write_endpoints() -> &'static [ApiEndpoint] {
    &WRITE_ENDPOINTS
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub body_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body_json: String,
}

pub fn handle_request(deps: &SoulDependencies, request: HttpRequest<'_>) -> HttpResponse {
    match dispatch_request(deps, request) {
        Ok(response) => response,
        Err(error) => error_response(&error),
    }
}

fn dispatch_request(
    deps: &SoulDependencies,
    request: HttpRequest<'_>,
) -> Result<HttpResponse, SoulError> {
    match (request.method, request.path) {
        (method, path)
            if method == UPDATE_TRAITS_ROUTE.method.as_str()
                && path == UPDATE_TRAITS_ROUTE.path =>
        {
            let body: UpdateTraitsRequest = parse_json(request.body_json)?;
            let config = handle_update_traits(deps, body)?;
            json_response(200, &config)
        }
        (method, path)
            if method == UPDATE_HEURISTICS_ROUTE.method.as_str()
                && path == UPDATE_HEURISTICS_ROUTE.path =>
        {
            let body: UpdateHeuristicsRequest = parse_json(request.body_json)?;
            let config = handle_update_heuristics(deps, body)?;
            json_response(200, &config)
        }
        (method, path)
            if method == RECORD_INTERACTION_ROUTE.method.as_str()
                && path == RECORD_INTERACTION_ROUTE.path =>
        {
            let body: RecordInteractionRequest = parse_json(request.body_json)?;
            let result = handle_record_interaction(deps, body)?;
            json_response(record_success_status(&result), &result)
        }
        (method, path)
            if method == RESET_ADAPTATION_ROUTE.method.as_str()
                && path == RESET_ADAPTATION_ROUTE.path =>
        {
            let body: ResetAdaptationRequest = parse_json(request.body_json)?;
            let result = handle_reset_adaptation(deps, body)?;
            json_response(200, &result)
        }
        _ => Err(SoulError::Validation(format!(
            "unsupported REST endpoint {} {}",
            request.method, request.path
        ))),
    }
}

fn parse_json<T>(raw: &str) -> Result<T, SoulError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(raw)
        .map_err(|error| SoulError::Validation(format!("invalid JSON request body: {error}")))
}

fn json_response<T>(status: u16, value: &T) -> Result<HttpResponse, SoulError>
where
    T: Serialize,
{
    Ok(HttpResponse {
        status,
        body_json: serde_json::to_string(value)
            .map_err(|error| SoulError::Internal(error.to_string()))?,
    })
}

fn error_response(error: &SoulError) -> HttpResponse {
    if let SoulError::Validation(message) = error {
        if message.starts_with("unsupported REST endpoint ") {
            let body = serde_json::json!({
                "error": {
                    "code": "not-found",
                    "category": "request-validation",
                    "message": message,
                }
            });
            return HttpResponse {
                status: 404,
                body_json: body.to_string(),
            };
        }
    }

    let response =
        request_specific_error(error).unwrap_or_else(|| map_soul_error(error).http_response());

    match serde_json::to_string(&response.body) {
        Ok(body_json) => HttpResponse {
            status: response.status,
            body_json,
        },
        Err(serialization_error) => {
            fallback_error_response(&SoulError::Internal(serialization_error.to_string()))
        }
    }
}

fn request_specific_error(error: &SoulError) -> Option<SoulHttpErrorResponse> {
    match error {
        SoulError::Validation(message)
            if message.contains("event_id")
                || message.contains("interaction_type")
                || message.contains("agent_id") =>
        {
            Some(record_error_response(error))
        }
        SoulError::Validation(message)
            if message.contains("reset_id") || message.contains("target_key") =>
        {
            Some(reset_error_response(error))
        }
        _ => None,
    }
}

fn fallback_error_response(error: &SoulError) -> HttpResponse {
    let response = map_soul_error(error).http_response();
    let body_json = serde_json::to_string(&SoulHttpErrorBody {
        error: response.body.error,
    })
    .unwrap_or_else(|_| {
        "{\"error\":{\"code\":\"internal-failure\",\"category\":\"internal-failure\",\"message\":\"internal runtime failure\"}}"
            .to_owned()
    });

    HttpResponse {
        status: response.status,
        body_json,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::Value;

    use crate::{
        app::{config::WorkspacePaths, deps::AppDeps},
        storage::sqlite,
    };

    use super::{
        HttpRequest, RECORD_INTERACTION_ROUTE, RESET_ADAPTATION_ROUTE, UPDATE_HEURISTICS_ROUTE,
        UPDATE_TRAITS_ROUTE, handle_request,
    };

    #[test]
    fn patch_traits_endpoint_updates_workspace_config() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("api-traits");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        let body = serde_json::json!({
            "workspace_root": workspace.display().to_string(),
            "updates": { "verbosity": 0.82, "formality": 0.71 }
        })
        .to_string();

        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: UPDATE_TRAITS_ROUTE.method.as_str(),
                path: UPDATE_TRAITS_ROUTE.path,
                body_json: &body,
            },
        );

        assert_eq!(response.status, 200);
        let body: Value = serde_json::from_str(&response.body_json)?;
        assert_eq!(body["trait_baseline"]["verbosity"], 0.82);
        assert_eq!(body["trait_baseline"]["formality"], 0.71);
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn patch_heuristics_endpoint_requires_operations() -> Result<(), Box<dyn Error>> {
        let body = serde_json::json!({
            "workspace_root": "/tmp/workspace",
            "patch": {}
        })
        .to_string();

        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: UPDATE_HEURISTICS_ROUTE.method.as_str(),
                path: UPDATE_HEURISTICS_ROUTE.path,
                body_json: &body,
            },
        );

        assert_eq!(response.status, 400);
        let body: Value = serde_json::from_str(&response.body_json)?;
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("heuristics endpoint requires")
        );
        Ok(())
    }

    #[test]
    fn post_interactions_endpoint_records_event_and_returns_created() -> Result<(), Box<dyn Error>>
    {
        let workspace = test_workspace("api-interactions");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        let body = serde_json::json!({
            "workspace_root": workspace.display().to_string(),
            "event_id": "evt-1",
            "agent_id": "agent.alpha",
            "interaction_type": "review",
            "outcome": "positive",
            "context": {"source": "rest"},
            "persist": true
        })
        .to_string();

        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: RECORD_INTERACTION_ROUTE.method.as_str(),
                path: RECORD_INTERACTION_ROUTE.path,
                body_json: &body,
            },
        );

        assert_eq!(response.status, 201);
        let body: Value = serde_json::from_str(&response.body_json)?;
        assert_eq!(body["effect"], "Inserted");
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn post_interactions_endpoint_is_idempotent_for_duplicate_event_ids()
    -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("api-interactions-duplicate");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        let body = serde_json::json!({
            "workspace_root": workspace.display().to_string(),
            "event_id": "evt-dup-1",
            "agent_id": "agent.alpha",
            "interaction_type": "review",
            "outcome": "positive",
            "context": {"source": "rest"},
            "persist": true
        })
        .to_string();

        let first = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: RECORD_INTERACTION_ROUTE.method.as_str(),
                path: RECORD_INTERACTION_ROUTE.path,
                body_json: &body,
            },
        );
        let duplicate = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: RECORD_INTERACTION_ROUTE.method.as_str(),
                path: RECORD_INTERACTION_ROUTE.path,
                body_json: &body,
            },
        );

        assert_eq!(first.status, 201);
        assert_eq!(duplicate.status, 200);
        let duplicate_body: Value = serde_json::from_str(&duplicate.body_json)?;
        assert_eq!(duplicate_body["effect"], "Duplicate");
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn post_reset_endpoint_clears_state_and_preserves_interaction_evidence()
    -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("api-reset");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        let interaction_body = serde_json::json!({
            "workspace_root": workspace.display().to_string(),
            "event_id": "evt-reset-1",
            "agent_id": "agent.alpha",
            "interaction_type": "review",
            "outcome": "positive",
            "context": {"source": "rest"},
            "persist": true
        })
        .to_string();
        let interaction_response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: RECORD_INTERACTION_ROUTE.method.as_str(),
                path: RECORD_INTERACTION_ROUTE.path,
                body_json: &interaction_body,
            },
        );
        assert_eq!(interaction_response.status, 201);

        let reset_body = serde_json::json!({
            "workspace_root": workspace.display().to_string(),
            "reset_id": "reset-1",
            "agent_id": "agent.alpha",
            "scope": "all"
        })
        .to_string();
        let reset_response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: RESET_ADAPTATION_ROUTE.method.as_str(),
                path: RESET_ADAPTATION_ROUTE.path,
                body_json: &reset_body,
            },
        );

        assert_eq!(reset_response.status, 200);
        let reset_body: Value = serde_json::from_str(&reset_response.body_json)?;
        assert_eq!(reset_body["effect"], "Cleared");

        let conn = sqlite::open_database(WorkspacePaths::new(&workspace).adaptation_db_path())?;
        let event_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM interaction_events WHERE agent_id = ?1",
            rusqlite::params!["agent.alpha"],
            |row| row.get(0),
        )?;
        assert_eq!(event_count, 1);
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn unsupported_route_returns_not_found() -> Result<(), Box<dyn Error>> {
        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: "GET",
                path: "/v1/missing",
                body_json: "{}",
            },
        );

        assert_eq!(response.status, 404);
        let body: Value = serde_json::from_str(&response.body_json)?;
        assert_eq!(body["error"]["code"], "not-found");
        Ok(())
    }

    #[test]
    fn invalid_json_returns_validation_error() -> Result<(), Box<dyn Error>> {
        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: UPDATE_TRAITS_ROUTE.method.as_str(),
                path: UPDATE_TRAITS_ROUTE.path,
                body_json: "{",
            },
        );

        assert_eq!(response.status, 400);
        let body: Value = serde_json::from_str(&response.body_json)?;
        assert_eq!(body["error"]["code"], "request-validation");
        Ok(())
    }

    fn test_workspace(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}"))
    }

    fn cleanup_workspace(workspace: &Path) -> Result<(), std::io::Error> {
        if workspace.exists() {
            fs::remove_dir_all(workspace)?;
        }
        Ok(())
    }

    fn write_soul_config(
        workspace: &Path,
        agent_id: &str,
        profile_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(workspace.join(".soul"))?;
        fs::write(workspace.join(".soul").join("adaptation_log.jsonl"), "")?;
        let config = crate::domain::SoulConfig {
            agent_id: agent_id.to_owned(),
            profile_name: profile_name.to_owned(),
            ..crate::domain::SoulConfig::default()
        };
        fs::write(workspace.join("soul.toml"), toml::to_string(&config)?)?;
        Ok(())
    }
}
