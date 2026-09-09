use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque identifiers — all UUIDs per `data_models.md`.
pub type ConceptId = Uuid;
pub type LearnerId = Uuid;
pub type ConversationId = Uuid;
pub type MessageId = Uuid;
pub type RelationId = Uuid;
pub type CandidateId = Uuid;
pub type MutationId = Uuid;
pub type ReviewId = Uuid;
pub type ObservationId = Uuid;

/// Newtype wrappers can be added later if stronger typing is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id<T>(pub Uuid, std::marker::PhantomData<T>);

impl<T> Id<T> {
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self
    where
        T: Default,
    {
        Self(Uuid::new_v4(), std::marker::PhantomData)
    }
}
