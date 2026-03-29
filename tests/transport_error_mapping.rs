use agents_soul::{ComposeMode, SoulError, SoulErrorCategory, api, cli, map_soul_error, mcp};

#[test]
fn transport_matrix_matches_plan_exit_code_contract() {
    let validation = map_soul_error(&SoulError::Validation("bad request".into()));
    assert_eq!(validation.category, SoulErrorCategory::RequestValidation);
    assert_eq!(validation.cli_exit_code, 2);
    assert_eq!(validation.http_status, 400);
    assert_eq!(validation.code, "request-validation");

    let upstream_unavailable = map_soul_error(&SoulError::RegistryUnavailable);
    assert_eq!(
        upstream_unavailable.category,
        SoulErrorCategory::UpstreamUnavailable
    );
    assert_eq!(upstream_unavailable.cli_exit_code, 3);
    assert_eq!(upstream_unavailable.http_status, 503);
    assert_eq!(
        upstream_unavailable.compose_mode_hint,
        Some(ComposeMode::Degraded)
    );

    let fail_closed = map_soul_error(&SoulError::RevokedStanding);
    assert_eq!(fail_closed.category, SoulErrorCategory::FailClosed);
    assert_eq!(fail_closed.cli_exit_code, 4);
    assert_eq!(fail_closed.http_status, 403);
    assert_eq!(fail_closed.compose_mode_hint, Some(ComposeMode::FailClosed));

    let local_config = map_soul_error(&SoulError::InvalidConfig("missing soul.toml".into()));
    assert_eq!(local_config.category, SoulErrorCategory::LocalConfig);
    assert_eq!(local_config.cli_exit_code, 5);
    assert_eq!(local_config.http_status, 500);

    let storage = map_soul_error(&SoulError::Storage("sqlite busy".into()));
    assert_eq!(storage.category, SoulErrorCategory::StorageFailure);
    assert_eq!(storage.cli_exit_code, 6);
    assert_eq!(storage.http_status, 500);

    let upstream_invalid = map_soul_error(&SoulError::UpstreamInvalid {
        input: "identity-snapshot",
        message: "broken json".into(),
    });
    assert_eq!(
        upstream_invalid.category,
        SoulErrorCategory::UpstreamInvalid
    );
    assert_eq!(upstream_invalid.cli_exit_code, 7);
    assert_eq!(upstream_invalid.http_status, 502);

    let template = map_soul_error(&SoulError::TemplateRender {
        template: "prompt-prefix",
        message: "missing variable".into(),
    });
    assert_eq!(template.category, SoulErrorCategory::TemplateFailure);
    assert_eq!(template.cli_exit_code, 7);
    assert_eq!(template.http_status, 500);
}

#[test]
fn api_error_response_uses_shared_transport_matrix() {
    let error = SoulError::RevokedStanding;
    let mapped = map_soul_error(&error);
    let response = api::compose::compose_error_response(&error);

    assert_eq!(response.status, mapped.http_status);
    assert_eq!(response.body.error.code, mapped.code);
    assert_eq!(response.body.error.category, mapped.category);
    assert_eq!(response.body.error.message, mapped.message);
    assert_eq!(
        response.body.error.compose_mode_hint,
        mapped.compose_mode_hint
    );
}

#[test]
fn api_mutation_error_responses_use_shared_transport_matrix() {
    let error = SoulError::Storage("db locked".into());
    let mapped = map_soul_error(&error);

    let interaction_response = api::interactions::record_error_response(&error);
    assert_eq!(interaction_response.status, mapped.http_status);
    assert_eq!(interaction_response.body.error.code, mapped.code);
    assert_eq!(interaction_response.body.error.category, mapped.category);
    assert_eq!(interaction_response.body.error.message, mapped.message);
    assert_eq!(
        interaction_response.body.error.compose_mode_hint,
        mapped.compose_mode_hint
    );

    let reset_response = api::reset::reset_error_response(&error);
    assert_eq!(reset_response.status, mapped.http_status);
    assert_eq!(reset_response.body.error.code, mapped.code);
    assert_eq!(reset_response.body.error.category, mapped.category);
    assert_eq!(reset_response.body.error.message, mapped.message);
    assert_eq!(
        reset_response.body.error.compose_mode_hint,
        mapped.compose_mode_hint
    );
}

#[test]
fn mcp_error_response_uses_shared_transport_matrix() {
    let error = SoulError::UpstreamInvalid {
        input: "registry-verification",
        message: "bad payload".into(),
    };
    let mapped = map_soul_error(&error);
    let tool_error = mcp::tools::compose_tool_error(&error);

    assert_eq!(tool_error.code, mapped.mcp_error_name);
    assert_eq!(tool_error.message, mapped.message);
    assert_eq!(tool_error.data.error_code, mapped.mcp_error_code);
    assert_eq!(tool_error.data.category, mapped.category);
    assert_eq!(tool_error.data.http_status, mapped.http_status);
    assert_eq!(tool_error.data.cli_exit_code, mapped.cli_exit_code);
}

#[test]
fn cli_error_mapping_uses_shared_exit_codes() {
    let error = SoulError::Storage("db locked".into());
    let mapped = cli::compose::map_compose_error(&error);

    assert_eq!(mapped.code, "storage-failure");
    assert_eq!(mapped.cli_exit_code, 6);
    assert_eq!(mapped.exit_code(), std::process::ExitCode::from(6));
}
