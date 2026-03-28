pub fn render_trait_note(trait_name: &str, delta: f32, signal_count: u32, reason: &str) -> String {
    let direction = if delta >= 0.0 { "increased" } else { "reduced" };
    let signal_label = if signal_count == 1 {
        "signal"
    } else {
        "signals"
    };

    format!(
        "{trait_name} {direction} by {:.2} from {signal_count} recent {signal_label}; {reason}",
        delta.abs()
    )
}

pub fn render_heuristic_note(
    heuristic_id: &str,
    priority_delta: i32,
    enabled: Option<bool>,
    replaced: bool,
    signal_count: u32,
    reason: &str,
) -> String {
    let signal_label = if signal_count == 1 {
        "signal"
    } else {
        "signals"
    };
    let mut parts = Vec::new();

    if priority_delta != 0 {
        let direction = if priority_delta > 0 {
            "increased"
        } else {
            "reduced"
        };
        parts.push(format!("priority {direction} by {}", priority_delta.abs()));
    }
    if let Some(enabled) = enabled {
        parts.push(if enabled {
            "enabled".to_owned()
        } else {
            "disabled".to_owned()
        });
    }
    if replaced {
        parts.push("instruction replaced".to_owned());
    }
    if parts.is_empty() {
        parts.push("retained".to_owned());
    }

    format!(
        "Heuristic `{heuristic_id}` {} from {signal_count} recent {signal_label}; {reason}",
        parts.join(", ")
    )
}
