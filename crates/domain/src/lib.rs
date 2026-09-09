//! Pure domain types and invariants.
//!
//! Forbidden dependencies: Axum, SQLx, LLM SDKs, filesystem/network I/O.
//! See `architecture.md -> Module Boundaries` and `data_models.md`.

pub mod concept;
pub mod confidence;
pub mod errors;
pub mod ids;
pub mod learner_state;
pub mod relation;

pub use concept::*;
pub use confidence::*;
pub use errors::*;
pub use ids::*;
pub use learner_state::*;
pub use relation::*;
