use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use agents_soul::app::deps::ComposeClock;
use agents_soul::domain::{
    AdaptationState, CURRENT_SCHEMA_VERSION, ComposeRequest, InputProvenance, InputSourceKind,
    NormalizedIdentityInputs, NormalizedInputs, NormalizedRegistryInputs, NormalizedUpstreamInputs,
    RecoveryState, RegistryStatus, SessionIdentitySnapshot, SoulConfig, SoulError, SourceConfig,
    VerificationResult,
};
use agents_soul::sources::{
    cache::{CachedInputs, write_cached_inputs},
    identity::IdentityReader,
    registry::RegistryReader,
};
use agents_soul::{
    BehavioralContext, ComposeMode, CrateLayer, SoulDependencies, SoulErrorCategory, SoulRuntime,
    app::config::ApplicationConfig,
    core_layers, crate_layout,
    services::{provenance::ProvenanceHasher, templates::PromptTemplateRenderer},
    transport_layers,
};
use chrono::{DateTime, TimeZone, Utc};

#[test]
fn bootstrap_surface_exposes_core_contract_types() {
    let context = BehavioralContext::default();
    let runtime = SoulRuntime::default();
    let deps = SoulDependencies::default();
    let error = agents_soul::map_soul_error(&SoulError::RegistryUnavailable);

    assert!(context.system_prompt_prefix.is_empty());
    assert_eq!(
        context.status_summary.compose_mode,
        ComposeMode::BaselineOnly
    );
    assert_eq!(
        runtime.config().workspace_paths().state_dir(),
        env::current_dir()
            .expect("cwd should resolve")
            .join(".soul")
    );
    assert_eq!(
        core_layers(),
        vec![
            CrateLayer::App,
            CrateLayer::Domain,
            CrateLayer::Sources,
            CrateLayer::Services,
            CrateLayer::Adaptation,
            CrateLayer::Storage,
        ]
    );
    assert_eq!(
        transport_layers(),
        vec![CrateLayer::Cli, CrateLayer::Api, CrateLayer::Mcp]
    );
    assert_eq!(crate_layout().len(), 9);
    assert_eq!(deps.sources.identity, Default::default());
    assert_eq!(deps.sources.registry, Default::default());
    assert_eq!(error.category, SoulErrorCategory::UpstreamUnavailable);
    assert_eq!(error.compose_mode_hint, Some(ComposeMode::Degraded));
}

#[derive(Debug, Clone)]
struct FixedClock;

impl ComposeClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 29, 8, 0, 0)
            .single()
            .expect("fixed timestamp should be valid")
    }
}

#[derive(Debug, Clone)]
struct FixedRenderer;

impl PromptTemplateRenderer for FixedRenderer {
    fn render_prompt_prefix(
        &self,
        template_name: &str,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> Result<String, SoulError> {
        Ok(format!(
            "runtime:{template_name}:{compose_mode:?}:{profile_name}:{max_chars}"
        ))
    }
}

#[derive(Debug, Clone)]
struct FixedHasher;

impl ProvenanceHasher for FixedHasher {
    fn identity_fingerprint(
        &self,
        _snapshot: &SessionIdentitySnapshot,
    ) -> Result<String, SoulError> {
        Ok("id_runtime".to_owned())
    }

    fn config_hash(&self, _config: &SoulConfig) -> Result<String, SoulError> {
        Ok("cfg_runtime".to_owned())
    }

    fn adaptation_hash(&self, _state: &AdaptationState) -> Result<String, SoulError> {
        Ok("adp_runtime".to_owned())
    }

    fn input_hash(&self, _normalized: &NormalizedInputs) -> Result<String, SoulError> {
        Ok("inp_runtime".to_owned())
    }
}

#[test]
fn runtime_dispatch_preserves_injected_config_and_deps() -> Result<(), SoulError> {
    let config = ApplicationConfig::new("/tmp/injected-soul");
    let deps = SoulDependencies::default()
        .with_clock(FixedClock)
        .with_template_renderer(FixedRenderer)
        .with_provenance_hasher(FixedHasher);
    let runtime = SoulRuntime::new(config.clone(), deps);

    runtime.dispatch_with(|seen_config, seen_deps| {
        let normalized = NormalizedInputs {
            schema_version: CURRENT_SCHEMA_VERSION,
            request: ComposeRequest::new("alpha", "session-1"),
            agent_id: "alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            compose_mode_hint: Some(ComposeMode::Normal),
            upstream: NormalizedUpstreamInputs {
                identity: NormalizedIdentityInputs {
                    snapshot: None,
                    recovery_state: None,
                    provenance: InputProvenance::unavailable("not loaded"),
                },
                registry: NormalizedRegistryInputs {
                    verification: None,
                    verification_provenance: InputProvenance::unavailable("not loaded"),
                    reputation: None,
                    reputation_provenance: InputProvenance::unavailable("not loaded"),
                },
            },
            identity_snapshot: None,
            identity_recovery_state: None,
            identity_provenance: InputProvenance::unavailable("not loaded"),
            verification_result: None,
            verification_provenance: InputProvenance::unavailable("not loaded"),
            reputation_summary: None,
            reputation_provenance: InputProvenance::unavailable("not loaded"),
            soul_config: SoulConfig::default(),
            adaptation_state: AdaptationState::default(),
            reader_warnings: Vec::new(),
            generated_at: seen_deps.now(),
        };

        assert_eq!(seen_config.workspace_root(), config.workspace_root());
        assert_eq!(
            seen_deps.now(),
            Utc.with_ymd_and_hms(2026, 3, 29, 8, 0, 0)
                .single()
                .expect("fixed timestamp should be valid")
        );
        assert_eq!(
            seen_deps.render_prompt_prefix(
                "prompt-prefix",
                ComposeMode::Restricted,
                "Alpha",
                32
            )?,
            "runtime:prompt-prefix:Restricted:Alpha:32"
        );
        assert_eq!(
            seen_deps
                .provenance_hasher()
                .config_hash(&SoulConfig::default())?,
            "cfg_runtime"
        );
        assert_eq!(
            seen_deps
                .provenance_hasher()
                .adaptation_hash(&AdaptationState::default())?,
            "adp_runtime"
        );
        assert_eq!(
            seen_deps.provenance_hasher().input_hash(&normalized)?,
            "inp_runtime"
        );
        Ok(())
    })
}

#[test]
fn bootstrap_identity_reader_prefers_live_identify_signals_over_live_snapshot_and_cache()
-> Result<(), SoulError> {
    let workspace = test_workspace("bootstrap-identify-precedence");
    let identity_workspace = workspace.join("identity");
    fs::create_dir_all(identity_workspace.join(".soul")).map_err(io_to_soul)?;
    fs::create_dir_all(workspace.join(".soul")).map_err(io_to_soul)?;

    fs::write(
        identity_workspace.join("session_identity_snapshot.json"),
        r#"{
            "agent_id":"alpha",
            "recovery_state":"healthy",
            "active_commitments":["live-snapshot"]
        }"#,
    )
    .map_err(io_to_soul)?;
    fs::write(
        identity_workspace.join("agents_identify.json"),
        r#"{
            "recovery_state":"degraded"
        }"#,
    )
    .map_err(io_to_soul)?;

    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();
    write_cached_inputs(
        &request,
        &CachedInputs {
            cache_key: None,
            identity_snapshot: Some(SessionIdentitySnapshot {
                agent_id: "alpha".to_owned(),
                display_name: Some("Alpha".to_owned()),
                recovery_state: RecoveryState::Healthy,
                active_commitments: vec!["cache".to_owned()],
                durable_preferences: Vec::new(),
                relationship_markers: Vec::new(),
                facts: Vec::new(),
                warnings: Vec::new(),
                fingerprint: None,
            }),
            verification_result: None,
            reputation_summary: None,
        },
    )?;

    let config = SoulConfig {
        agent_id: "alpha".to_owned(),
        profile_name: "Alpha".to_owned(),
        sources: SourceConfig {
            identity_workspace: identity_workspace.display().to_string(),
            ..SoulConfig::default().sources
        },
        ..SoulConfig::default()
    };

    let selection = IdentityReader.load(&request, &config)?;
    let signals = selection.value.expect("identify signals should load");

    assert_eq!(selection.provenance.source, InputSourceKind::Live);
    assert_eq!(signals.recovery_state, Some(RecoveryState::Degraded));
    assert!(signals.snapshot.is_none());

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn bootstrap_registry_reader_prefers_explicit_verification_over_live_and_cache()
-> Result<(), SoulError> {
    let workspace = test_workspace("bootstrap-registry-precedence");
    fs::create_dir_all(workspace.join(".soul")).map_err(io_to_soul)?;

    fs::write(
        workspace.join("registry_verification.json"),
        r#"{
            "status":"suspended",
            "standing_level":"live"
        }"#,
    )
    .map_err(io_to_soul)?;
    fs::write(
        workspace.join("explicit_verification.json"),
        r#"{
            "status":"active",
            "standing_level":"explicit"
        }"#,
    )
    .map_err(io_to_soul)?;

    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();
    request.registry_verification_path = Some(
        workspace
            .join("explicit_verification.json")
            .display()
            .to_string(),
    );
    write_cached_inputs(
        &request,
        &CachedInputs {
            cache_key: None,
            identity_snapshot: None,
            verification_result: Some(VerificationResult {
                status: RegistryStatus::Revoked,
                standing_level: Some("cache".to_owned()),
                reason_code: None,
                verified_at: None,
            }),
            reputation_summary: None,
        },
    )?;

    let selection = RegistryReader::default().load_verification(&request)?;
    let verification = selection.value.expect("verification should load");

    assert_eq!(selection.provenance.source, InputSourceKind::Explicit);
    assert_eq!(verification.status, RegistryStatus::Active);
    assert_eq!(verification.standing_level.as_deref(), Some("explicit"));

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn bootstrap_utility_helpers_are_deterministic() -> Result<(), SoulError> {
    let first_hash = agents_soul::app::hash::stable_content_hash("agents-soul");
    let second_hash = agents_soul::app::hash::stable_content_hash("agents-soul");
    assert_eq!(first_hash, second_hash);
    assert_eq!(first_hash.len(), 64);

    let sorted = agents_soul::app::hash::sorted_dedup_strings(vec![
        "zeta".to_owned(),
        "alpha".to_owned(),
        "zeta".to_owned(),
        "beta".to_owned(),
    ]);
    assert_eq!(sorted, vec!["alpha", "beta", "zeta"]);

    let hasher = agents_soul::services::provenance::StableProvenanceHasher;
    let snapshot = SessionIdentitySnapshot {
        agent_id: "alpha".to_owned(),
        display_name: Some("Alpha".to_owned()),
        recovery_state: RecoveryState::Healthy,
        active_commitments: vec!["commit-a".to_owned(), "commit-b".to_owned()],
        durable_preferences: vec!["prefer-terse".to_owned()],
        relationship_markers: vec!["trusted-reviewer".to_owned()],
        facts: vec!["fact-1".to_owned()],
        warnings: vec!["warning-1".to_owned()],
        fingerprint: None,
    };

    let first_fingerprint = hasher.identity_fingerprint(&snapshot)?;
    let second_fingerprint = hasher.identity_fingerprint(&snapshot)?;
    assert_eq!(first_fingerprint, second_fingerprint);
    assert!(first_fingerprint.starts_with("id_"));

    Ok(())
}

fn test_workspace(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}"))
}

fn cleanup_workspace(workspace: &Path) -> Result<(), SoulError> {
    if workspace.exists() {
        fs::remove_dir_all(workspace).map_err(io_to_soul)?;
    }
    Ok(())
}

fn io_to_soul(error: std::io::Error) -> SoulError {
    SoulError::Storage(error.to_string())
}
