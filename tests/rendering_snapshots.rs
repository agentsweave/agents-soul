use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use agents_soul::app::deps::ComposeClock;
use agents_soul::{AppDeps, ComposeRequest, SoulError};
use chrono::{DateTime, TimeZone, Utc};

#[derive(Debug, Clone)]
struct FixedClock;

impl ComposeClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 29, 8, 0, 0)
            .single()
            .expect("fixed timestamp should be valid")
    }
}

#[derive(Clone, Copy)]
struct SnapshotCase {
    name: &'static str,
    identity_json: &'static str,
    verification_json: &'static str,
    reputation_json: &'static str,
}

#[test]
fn rendering_snapshots_match_expected_fixtures() -> Result<(), Box<dyn Error>> {
    for case in snapshot_cases() {
        let actual = render_snapshot(case)?;
        assert_snapshot(case.name, &actual)?;
    }

    Ok(())
}

fn snapshot_cases() -> [SnapshotCase; 4] {
    [
        SnapshotCase {
            name: "normal",
            identity_json: r#"{
                "snapshot": {
                    "agent_id":"agent.alpha",
                    "display_name":"Alpha Builder",
                    "recovery_state":"healthy",
                    "active_commitments":["Ship precise answers","Protect operator trust"],
                    "relationship_markers":[
                        {"subject":"operator","marker":"trusted","note":"primary owner"},
                        {"subject":"repo","marker":"maintainer","note":"owns release lane"}
                    ],
                    "facts":["prefers concise updates"],
                    "warnings":[],
                    "fingerprint":"id-normal"
                },
                "recovery_state":"healthy"
            }"#,
            verification_json: r#"{
                "status":"active",
                "standing_level":"good",
                "reason_code":null,
                "verified_at":"2026-03-29T08:00:00Z"
            }"#,
            reputation_json: r#"{
                "score_total":4.8,
                "score_recent_30d":4.7,
                "last_event_at":"2026-03-28T10:00:00Z",
                "context":["trusted operator feedback","clean recent history"]
            }"#,
        },
        SnapshotCase {
            name: "restricted",
            identity_json: r#"{
                "snapshot": {
                    "agent_id":"agent.alpha",
                    "display_name":"Alpha Builder",
                    "recovery_state":"healthy",
                    "active_commitments":["Protect operator trust"],
                    "relationship_markers":[
                        {"subject":"operator","marker":"trusted","note":"primary owner"}
                    ],
                    "facts":["must escalate risky work"],
                    "warnings":[],
                    "fingerprint":"id-restricted"
                },
                "recovery_state":"healthy"
            }"#,
            verification_json: r#"{
                "status":"suspended",
                "standing_level":"watch",
                "reason_code":"manual-review",
                "verified_at":"2026-03-29T08:00:00Z"
            }"#,
            reputation_json: r#"{
                "score_total":2.1,
                "score_recent_30d":1.8,
                "last_event_at":"2026-03-27T14:30:00Z",
                "context":["manual review","recent policy incident"]
            }"#,
        },
        SnapshotCase {
            name: "degraded",
            identity_json: r#"{
                "snapshot": {
                    "agent_id":"agent.alpha",
                    "display_name":"Alpha Builder",
                    "recovery_state":"degraded",
                    "active_commitments":["Keep operator informed"],
                    "relationship_markers":[
                        {"subject":"operator","marker":"trusted","note":"primary owner"}
                    ],
                    "facts":["upstream health is degraded"],
                    "warnings":[],
                    "fingerprint":"id-degraded"
                },
                "recovery_state":"degraded"
            }"#,
            verification_json: r#"{
                "status":"active",
                "standing_level":"good",
                "reason_code":null,
                "verified_at":"2026-03-29T08:00:00Z"
            }"#,
            reputation_json: r#"{
                "score_total":3.9,
                "score_recent_30d":3.2,
                "last_event_at":"2026-03-28T11:00:00Z",
                "context":["degraded upstream telemetry","fallback confidence reduced"]
            }"#,
        },
        SnapshotCase {
            name: "fail_closed",
            identity_json: r#"{
                "snapshot": {
                    "agent_id":"agent.alpha",
                    "display_name":"Alpha Builder",
                    "recovery_state":"healthy",
                    "active_commitments":["Protect operator trust"],
                    "relationship_markers":[
                        {"subject":"operator","marker":"trusted","note":"primary owner"}
                    ],
                    "facts":["revocation must halt autonomy"],
                    "warnings":[],
                    "fingerprint":"id-fail-closed"
                },
                "recovery_state":"healthy"
            }"#,
            verification_json: r#"{
                "status":"revoked",
                "standing_level":"revoked",
                "reason_code":"fraud-check",
                "verified_at":"2026-03-29T08:00:00Z"
            }"#,
            reputation_json: r#"{
                "score_total":0.4,
                "score_recent_30d":0.2,
                "last_event_at":"2026-03-29T07:45:00Z",
                "context":["standing revoked","operator escalation required"]
            }"#,
        },
    ]
}

fn render_snapshot(case: SnapshotCase) -> Result<String, Box<dyn Error>> {
    let workspace = test_workspace(case.name);
    if workspace.exists() {
        fs::remove_dir_all(&workspace)?;
    }
    fs::create_dir_all(&workspace)?;
    fs::write(
        workspace.join("soul.toml"),
        include_str!("../examples/workspaces/healthy/soul.toml"),
    )?;

    let identity_path = workspace.join("identity.json");
    let verification_path = workspace.join("registry_verification.json");
    let reputation_path = workspace.join("registry_reputation.json");
    fs::write(&identity_path, case.identity_json)?;
    fs::write(&verification_path, case.verification_json)?;
    fs::write(&reputation_path, case.reputation_json)?;

    let mut request = ComposeRequest::new("agent.alpha", &format!("snapshot-{}", case.name));
    request.workspace_id = workspace.display().to_string();
    request.identity_snapshot_path = Some(identity_path.display().to_string());
    request.registry_verification_path = Some(verification_path.display().to_string());
    request.registry_reputation_path = Some(reputation_path.display().to_string());

    let deps = AppDeps::default().with_clock(FixedClock);
    let context = deps.compose_context(request.clone())?;
    let full_context = deps.full_context_report(request.clone())?;
    let explain = deps.explain_report(request)?;

    let snapshot = format!(
        "# {name}\n\n## System Prompt Prefix\n{prefix}\n\n## Full Context\n{full_context}\n\n## Explain\n{explain}\n",
        name = case.name,
        prefix = context.system_prompt_prefix,
        full_context = full_context.rendered,
        explain = explain.rendered,
    );

    cleanup_workspace(&workspace)?;
    Ok(snapshot)
}

fn assert_snapshot(name: &str, actual: &str) -> Result<(), Box<dyn Error>> {
    let path = fixture_dir().join(format!("{name}.md"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        fs::create_dir_all(path.parent().expect("fixture dir"))?;
        fs::write(&path, actual)?;
    }

    let expected = fs::read_to_string(&path).map_err(|error| SoulError::ConfigRead {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    assert_eq!(actual, expected, "snapshot mismatch for {name}");
    Ok(())
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rendering")
}

fn test_workspace(label: &str) -> PathBuf {
    std::env::temp_dir()
        .join("agents-soul-rendering")
        .join(label)
}

fn cleanup_workspace(workspace: &Path) -> Result<(), Box<dyn Error>> {
    if workspace.exists() {
        fs::remove_dir_all(workspace)?;
    }
    Ok(())
}
