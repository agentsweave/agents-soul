use agents_soul::{
    ComposeMode, SoulDependencies, SoulError, api, app, cli, domain::ComposeRequest, mcp,
};

#[test]
fn crate_layout_matches_planned_boundary_order() -> Result<(), String> {
    let layout = app::runtime::crate_layout()
        .iter()
        .map(|layer| layer.name())
        .collect::<Vec<_>>();

    let expected_layout = vec![
        "app",
        "domain",
        "sources",
        "services",
        "adaptation",
        "storage",
        "cli",
        "api",
        "mcp",
    ];

    if layout != expected_layout {
        return Err(format!(
            "unexpected crate layout: expected {expected_layout:?}, got {layout:?}"
        ));
    }

    let transport_layers = app::runtime::transport_layers()
        .iter()
        .map(|layer| layer.name())
        .collect::<Vec<_>>();
    let expected_transport = vec!["cli", "api", "mcp"];

    if transport_layers != expected_transport {
        return Err(format!(
            "unexpected transport layers: expected {expected_transport:?}, got {transport_layers:?}"
        ));
    }

    Ok(())
}

#[test]
fn api_and_mcp_delegate_to_the_shared_compose_service() -> Result<(), String> {
    let deps = SoulDependencies::default();
    let request = ComposeRequest::new("agent.alpha", "session.alpha");
    let expected = deps
        .compose_context(request.clone())
        .map_err(|error| error.to_string())?;
    let api_result =
        api::compose::compose_context(&deps, request.clone()).map_err(|error| error.to_string())?;
    let mcp_result =
        mcp::tools::compose_context(&deps, request).map_err(|error| error.to_string())?;

    if api_result != expected {
        return Err(format!(
            "api compose diverged from shared service: expected {expected:?}, got {api_result:?}"
        ));
    }

    if mcp_result != expected {
        return Err(format!(
            "mcp compose diverged from shared service: expected {expected:?}, got {mcp_result:?}"
        ));
    }

    Ok(())
}

#[test]
fn transports_share_the_same_core_error_mapping() {
    let deps = SoulDependencies::default();
    let degraded = SoulError::RegistryUnavailable;
    let degraded_expected = degraded.transport_error();

    assert_eq!(deps.map_error(&degraded), degraded_expected);
    assert_eq!(
        cli::compose::map_compose_error(&degraded),
        degraded_expected
    );
    assert_eq!(
        api::compose::map_compose_error(&degraded),
        degraded_expected
    );
    assert_eq!(mcp::tools::map_compose_error(&degraded), degraded_expected);
    assert_eq!(
        degraded_expected.compose_mode_hint,
        Some(ComposeMode::Degraded)
    );

    let fail_closed = SoulError::RevokedStanding;
    let fail_closed_expected = fail_closed.transport_error();

    assert_eq!(
        fail_closed_expected.compose_mode_hint,
        Some(ComposeMode::FailClosed)
    );
    assert_eq!(
        cli::compose::map_compose_error(&fail_closed),
        fail_closed_expected
    );
}
