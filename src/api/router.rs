use serde::{Deserialize, Serialize};

use crate::{
    api::{
        compose::compose_context,
        explain::explain_report,
        heuristics::{UpdateHeuristicsRequest, handle_update_heuristics, heuristics_projection},
        interactions::{
            RecordInteractionRequest, handle_record_interaction, record_error_response,
            record_success_status,
        },
        reset::{ResetAdaptationRequest, handle_reset_adaptation, reset_error_response},
        traits::{UpdateTraitsRequest, handle_update_traits, traits_projection},
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
pub const COMPOSE_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Post,
    path: "/v1/compose",
    handler: "api::compose::compose_context",
};
pub const EXPLAIN_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Post,
    path: "/v1/explain",
    handler: "api::explain::explain_report",
};
pub const READ_TRAITS_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Post,
    path: "/v1/traits",
    handler: "api::traits::traits_projection",
};
pub const READ_HEURISTICS_ROUTE: ApiEndpoint = ApiEndpoint {
    method: HttpMethod::Post,
    path: "/v1/heuristics",
    handler: "api::heuristics::heuristics_projection",
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
const READ_ENDPOINTS: [ApiEndpoint; 4] = [
    COMPOSE_ROUTE,
    EXPLAIN_ROUTE,
    READ_TRAITS_ROUTE,
    READ_HEURISTICS_ROUTE,
];

pub fn write_endpoints() -> &'static [ApiEndpoint] {
    &WRITE_ENDPOINTS
}

pub fn read_endpoints() -> &'static [ApiEndpoint] {
    &READ_ENDPOINTS
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
        (method, path) if method == COMPOSE_ROUTE.method.as_str() && path == COMPOSE_ROUTE.path => {
            let body: crate::domain::ComposeRequest = parse_json(request.body_json)?;
            let context = compose_context(deps, body)?;
            json_response(200, &context)
        }
        (method, path) if method == EXPLAIN_ROUTE.method.as_str() && path == EXPLAIN_ROUTE.path => {
            let body: crate::domain::ComposeRequest = parse_json(request.body_json)?;
            let report = explain_report(deps, body)?;
            json_response(200, &report)
        }
        (method, path)
            if method == READ_TRAITS_ROUTE.method.as_str() && path == READ_TRAITS_ROUTE.path =>
        {
            let body: crate::domain::ComposeRequest = parse_json(request.body_json)?;
            let traits = traits_projection(deps, body)?;
            json_response(200, &traits)
        }
        (method, path)
            if method == READ_HEURISTICS_ROUTE.method.as_str()
                && path == READ_HEURISTICS_ROUTE.path =>
        {
            let body: crate::domain::ComposeRequest = parse_json(request.body_json)?;
            let heuristics = heuristics_projection(deps, body)?;
            json_response(200, &heuristics)
        }
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
        COMPOSE_ROUTE, EXPLAIN_ROUTE, HttpRequest, READ_HEURISTICS_ROUTE, READ_TRAITS_ROUTE,
        RECORD_INTERACTION_ROUTE, RESET_ADAPTATION_ROUTE, UPDATE_HEURISTICS_ROUTE,
        UPDATE_TRAITS_ROUTE, handle_request,
    };

    #[test]
    fn post_compose_endpoint_returns_behavioral_context() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("api-compose");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let identity_path = workspace.join("identity.json");
        let verification_path = workspace.join("verification.json");
        fs::write(
            &identity_path,
            include_str!("../../tests/fixtures/compose_modes/identity_healthy.json"),
        )?;
        fs::write(
            &verification_path,
            include_str!("../../tests/fixtures/compose_modes/verification_active.json"),
        )?;

        let body = serde_json::json!({
            "workspace_id": workspace.display().to_string(),
            "agent_id": "agent.alpha",
            "session_id": "session.alpha",
            "identity_snapshot_path": identity_path.display().to_string(),
            "registry_verification_path": verification_path.display().to_string()
        })
        .to_string();

        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: COMPOSE_ROUTE.method.as_str(),
                path: COMPOSE_ROUTE.path,
                body_json: &body,
            },
        );

        ensure(
            response.status == 200,
            format!("expected 200, got {}", response.status),
        )?;
        let body: Value = serde_json::from_str(&response.body_json)?;
        ensure(
            body["status_summary"]["compose_mode"] == "normal",
            format!(
                "expected normal compose mode, got {:?}",
                body["status_summary"]["compose_mode"]
            ),
        )?;
        ensure(
            body["agent_id"] == "agent.alpha",
            format!("expected agent.alpha, got {:?}", body["agent_id"]),
        )?;
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn post_explain_endpoint_returns_explain_report() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("api-explain");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let identity_path = workspace.join("identity.json");
        let verification_path = workspace.join("verification.json");
        fs::write(
            &identity_path,
            include_str!("../../tests/fixtures/compose_modes/identity_degraded.json"),
        )?;
        fs::write(
            &verification_path,
            include_str!("../../tests/fixtures/compose_modes/verification_active.json"),
        )?;

        let body = serde_json::json!({
            "workspace_id": workspace.display().to_string(),
            "agent_id": "agent.alpha",
            "session_id": "session.alpha",
            "identity_snapshot_path": identity_path.display().to_string(),
            "registry_verification_path": verification_path.display().to_string()
        })
        .to_string();

        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: EXPLAIN_ROUTE.method.as_str(),
                path: EXPLAIN_ROUTE.path,
                body_json: &body,
            },
        );

        ensure(
            response.status == 200,
            format!("expected 200, got {}", response.status),
        )?;
        let body: Value = serde_json::from_str(&response.body_json)?;
        ensure(
            body["status_summary"]["compose_mode"] == "degraded",
            format!(
                "expected degraded compose mode, got {:?}",
                body["status_summary"]["compose_mode"]
            ),
        )?;
        ensure(
            body["inspect"]["status_summary"]["compose_mode"] == "degraded",
            format!(
                "expected degraded inspect mode, got {:?}",
                body["inspect"]["status_summary"]["compose_mode"]
            ),
        )?;
        ensure(
            body["rendered"]
                .as_str()
                .unwrap_or_default()
                .contains("Explain Alpha"),
            "expected rendered explain output to contain report title".to_owned(),
        )?;
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn post_traits_endpoint_returns_trait_projection() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("api-traits-read");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let identity_path = workspace.join("identity.json");
        let verification_path = workspace.join("verification.json");
        fs::write(
            &identity_path,
            include_str!("../../tests/fixtures/compose_modes/identity_healthy.json"),
        )?;
        fs::write(
            &verification_path,
            include_str!("../../tests/fixtures/compose_modes/verification_active.json"),
        )?;

        let body = serde_json::json!({
            "workspace_id": workspace.display().to_string(),
            "agent_id": "agent.alpha",
            "session_id": "session.alpha",
            "identity_snapshot_path": identity_path.display().to_string(),
            "registry_verification_path": verification_path.display().to_string()
        })
        .to_string();

        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: READ_TRAITS_ROUTE.method.as_str(),
                path: READ_TRAITS_ROUTE.path,
                body_json: &body,
            },
        );

        ensure(
            response.status == 200,
            format!("expected 200, got {}", response.status),
        )?;
        let actual: crate::services::explain::InspectTraitProjection =
            serde_json::from_str(&response.body_json)?;
        let expected = AppDeps::default()
            .inspect_report(crate::domain::ComposeRequest {
                workspace_id: workspace.display().to_string(),
                agent_id: "agent.alpha".to_owned(),
                session_id: "session.alpha".to_owned(),
                identity_snapshot_path: Some(identity_path.display().to_string()),
                registry_verification_path: Some(verification_path.display().to_string()),
                registry_reputation_path: None,
                include_reputation: true,
                include_relationships: true,
                include_commitments: true,
            })?
            .traits_only();
        ensure(
            actual == expected,
            "traits endpoint diverged from shared inspect projection".to_owned(),
        )?;
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn post_heuristics_endpoint_returns_heuristic_projection() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("api-heuristics-read");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let identity_path = workspace.join("identity.json");
        let verification_path = workspace.join("verification.json");
        fs::write(
            &identity_path,
            include_str!("../../tests/fixtures/compose_modes/identity_healthy.json"),
        )?;
        fs::write(
            &verification_path,
            include_str!("../../tests/fixtures/compose_modes/verification_active.json"),
        )?;

        let body = serde_json::json!({
            "workspace_id": workspace.display().to_string(),
            "agent_id": "agent.alpha",
            "session_id": "session.alpha",
            "identity_snapshot_path": identity_path.display().to_string(),
            "registry_verification_path": verification_path.display().to_string()
        })
        .to_string();

        let response = handle_request(
            &AppDeps::default(),
            HttpRequest {
                method: READ_HEURISTICS_ROUTE.method.as_str(),
                path: READ_HEURISTICS_ROUTE.path,
                body_json: &body,
            },
        );

        ensure(
            response.status == 200,
            format!("expected 200, got {}", response.status),
        )?;
        let actual: crate::services::explain::InspectHeuristicProjection =
            serde_json::from_str(&response.body_json)?;
        let expected = AppDeps::default()
            .inspect_report(crate::domain::ComposeRequest {
                workspace_id: workspace.display().to_string(),
                agent_id: "agent.alpha".to_owned(),
                session_id: "session.alpha".to_owned(),
                identity_snapshot_path: Some(identity_path.display().to_string()),
                registry_verification_path: Some(verification_path.display().to_string()),
                registry_reputation_path: None,
                include_reputation: true,
                include_relationships: true,
                include_commitments: true,
            })?
            .heuristics_only();
        ensure(
            actual == expected,
            "heuristics endpoint diverged from shared inspect projection".to_owned(),
        )?;
        cleanup_workspace(&workspace)?;
        Ok(())
    }

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

        ensure(
            response.status == 200,
            format!("expected 200, got {}", response.status),
        )?;
        let body: Value = serde_json::from_str(&response.body_json)?;
        ensure(
            body["trait_baseline"]["verbosity"] == 0.82,
            format!(
                "expected verbosity 0.82, got {:?}",
                body["trait_baseline"]["verbosity"]
            ),
        )?;
        ensure(
            body["trait_baseline"]["formality"] == 0.71,
            format!(
                "expected formality 0.71, got {:?}",
                body["trait_baseline"]["formality"]
            ),
        )?;
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

        ensure(
            response.status == 400,
            format!("expected 400, got {}", response.status),
        )?;
        let body: Value = serde_json::from_str(&response.body_json)?;
        ensure(
            body["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("heuristics endpoint requires"),
            format!("unexpected error message: {:?}", body["error"]["message"]),
        )?;
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

        ensure(
            response.status == 201,
            format!("expected 201, got {}", response.status),
        )?;
        let body: Value = serde_json::from_str(&response.body_json)?;
        ensure(
            body["effect"] == "Inserted",
            format!("expected Inserted effect, got {:?}", body["effect"]),
        )?;
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

        ensure(
            first.status == 201,
            format!("expected 201, got {}", first.status),
        )?;
        ensure(
            duplicate.status == 200,
            format!("expected 200, got {}", duplicate.status),
        )?;
        let duplicate_body: Value = serde_json::from_str(&duplicate.body_json)?;
        ensure(
            duplicate_body["effect"] == "Duplicate",
            format!(
                "expected Duplicate effect, got {:?}",
                duplicate_body["effect"]
            ),
        )?;
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
        ensure(
            interaction_response.status == 201,
            format!("expected 201, got {}", interaction_response.status),
        )?;

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

        ensure(
            reset_response.status == 200,
            format!("expected 200, got {}", reset_response.status),
        )?;
        let reset_body: Value = serde_json::from_str(&reset_response.body_json)?;
        ensure(
            reset_body["effect"] == "Cleared",
            format!("expected Cleared effect, got {:?}", reset_body["effect"]),
        )?;

        let conn = sqlite::open_database(WorkspacePaths::new(&workspace).adaptation_db_path())?;
        let event_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM interaction_events WHERE agent_id = ?1",
            rusqlite::params!["agent.alpha"],
            |row| row.get(0),
        )?;
        ensure(
            event_count == 1,
            format!("expected 1 preserved interaction event, got {event_count}"),
        )?;
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

        ensure(
            response.status == 404,
            format!("expected 404, got {}", response.status),
        )?;
        let body: Value = serde_json::from_str(&response.body_json)?;
        ensure(
            body["error"]["code"] == "not-found",
            format!("expected not-found code, got {:?}", body["error"]["code"]),
        )?;
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

        ensure(
            response.status == 400,
            format!("expected 400, got {}", response.status),
        )?;
        let body: Value = serde_json::from_str(&response.body_json)?;
        ensure(
            body["error"]["code"] == "request-validation",
            format!(
                "expected request-validation code, got {:?}",
                body["error"]["code"]
            ),
        )?;
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

    fn ensure(condition: bool, message: String) -> Result<(), Box<dyn Error>> {
        if condition {
            Ok(())
        } else {
            Err(message.into())
        }
    }
}
