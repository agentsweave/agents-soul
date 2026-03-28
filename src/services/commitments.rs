use crate::domain::NormalizedInputs;

#[derive(Debug, Clone, Default)]
pub struct CommitmentsService;

impl CommitmentsService {
    pub fn derive(&self, normalized: &NormalizedInputs) -> Vec<String> {
        normalized
            .identity_snapshot
            .as_ref()
            .map(|snapshot| snapshot.active_commitments.clone())
            .unwrap_or_default()
    }
}
