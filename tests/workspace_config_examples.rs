use std::{error::Error, fs, path::PathBuf};

use agents_soul::domain::{OfflineRegistryBehavior, SoulConfig};

#[test]
fn minimal_workspace_example_loads_and_applies_registry_agent_default() -> Result<(), Box<dyn Error>>
{
    let config = load_example("minimal/soul.toml")?;
    assert_eq!(config.agent_id, "agent.alpha");
    assert_eq!(config.sources.registry_agent_id, config.agent_id);
    Ok(())
}

#[test]
fn healthy_workspace_example_loads_full_authoring_surface() -> Result<(), Box<dyn Error>> {
    let config = load_example("healthy/soul.toml")?;
    assert_eq!(config.profile_name, "Alpha Builder");
    assert_eq!(config.decision_heuristics.len(), 2);
    assert_eq!(config.sources.registry_agent_id, "agent.alpha");
    Ok(())
}

#[test]
fn degraded_workspace_example_sets_baseline_only_offline_behavior() -> Result<(), Box<dyn Error>> {
    let config = load_example("degraded/soul.toml")?;
    assert_eq!(
        config.limits.offline_registry_behavior,
        OfflineRegistryBehavior::BaselineOnly
    );
    assert!(!config.adaptation.enabled);
    Ok(())
}

fn load_example(relative: &str) -> Result<SoulConfig, Box<dyn Error>> {
    let path = workspace_examples_root().join(relative);
    let raw = fs::read_to_string(&path)?;
    let parsed = toml::from_str::<SoulConfig>(&raw)?;
    Ok(parsed.finalize()?)
}

fn workspace_examples_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/workspaces")
}
