use agents_soul::{api, app, domain::ComposeRequest, mcp, services::SoulServices};

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
    let services = SoulServices::default();
    let request = ComposeRequest::new("agent.alpha", "session.alpha");
    let expected = services
        .compose
        .compose(request.clone())
        .map_err(|error| error.to_string())?;
    let api_result = api::compose::compose_context(&services, request.clone())
        .map_err(|error| error.to_string())?;
    let mcp_result =
        mcp::tools::compose_context(&services, request).map_err(|error| error.to_string())?;

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
