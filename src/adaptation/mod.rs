pub mod bounds;
pub mod ema;
pub mod notes;
pub mod overrides;
pub mod reset;
pub mod store;

use crate::adaptation::{
    bounds::AdaptationBounds, ema::EmaReducer, notes::AdaptiveNotes,
    overrides::OverrideMaterializer, reset::AdaptationReset, store::AdaptationStore,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdaptationStack {
    pub store: AdaptationStore,
    pub ema: EmaReducer,
    pub bounds: AdaptationBounds,
    pub overrides: OverrideMaterializer,
    pub reset: AdaptationReset,
    pub notes: AdaptiveNotes,
}
