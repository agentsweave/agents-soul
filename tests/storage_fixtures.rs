use std::error::Error;

use agents_soul::storage::{
    fixtures::{StorageFixture, minimal_adaptation_state_record, timestamp_utc},
    sqlite::{load_adaptation_state, upsert_adaptation_state},
};

#[test]
fn storage_fixture_spins_up_isolated_sqlite_dbs_deterministically() -> Result<(), Box<dyn Error>> {
    let updated_at = timestamp_utc(2026, 3, 29, 0, 0, 0)?;

    let fixture_a = StorageFixture::new("storage-fixture-a")?;
    let fixture_b = StorageFixture::new("storage-fixture-b")?;

    let conn_a = fixture_a.open_adaptation_db()?;
    let conn_b = fixture_b.open_adaptation_db()?;

    let mut record_a = minimal_adaptation_state_record("agent.alpha", updated_at);
    record_a.trait_overrides_json = r#"{"verbosity":0.25}"#.to_owned();

    let mut record_b = minimal_adaptation_state_record("agent.alpha", updated_at);
    record_b.trait_overrides_json = r#"{"verbosity":0.75}"#.to_owned();

    upsert_adaptation_state(&conn_a, &record_a)?;
    upsert_adaptation_state(&conn_b, &record_b)?;

    let loaded_a = load_adaptation_state(&conn_a, "agent.alpha")?.expect("state a");
    let loaded_b = load_adaptation_state(&conn_b, "agent.alpha")?.expect("state b");

    assert_eq!(loaded_a.trait_overrides_json, r#"{"verbosity":0.25}"#);
    assert_eq!(loaded_b.trait_overrides_json, r#"{"verbosity":0.75}"#);
    assert_ne!(
        fixture_a.adaptation_db_path(),
        fixture_b.adaptation_db_path()
    );

    Ok(())
}
