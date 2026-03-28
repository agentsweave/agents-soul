use crate::domain::NormalizedInputs;

#[derive(Debug, Clone, Default)]
pub struct RelationshipsService;

impl RelationshipsService {
    pub fn derive(&self, normalized: &NormalizedInputs) -> Vec<String> {
        normalized
            .identity_snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .relationship_markers
                    .iter()
                    .map(|marker| match &marker.note {
                        Some(note) => format!("{}:{} ({note})", marker.subject, marker.marker),
                        None => format!("{}:{}", marker.subject, marker.marker),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
