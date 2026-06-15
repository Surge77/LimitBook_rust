//! Operational errors returned from the engine API.

use crate::domain::ids::OrderId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Operational failures from engine commands — as opposed to
/// [`crate::domain::event::RejectReason`], which describes well-formed orders the engine
/// declined to accept for business reasons.
///
/// Library code returns this via `Result` and never panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EngineError {
    /// A cancel or amend referenced an order id the book does not contain.
    UnknownOrder(OrderId),
    /// An amend requested a non-positive quantity.
    InvalidAmendQuantity,
    /// The internal order arena is full (capacity exhausted). Indicates misconfiguration.
    CapacityExhausted,
}

impl core::fmt::Display for EngineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EngineError::UnknownOrder(id) => write!(f, "unknown order: {id}"),
            EngineError::InvalidAmendQuantity => write!(f, "amend quantity must be positive"),
            EngineError::CapacityExhausted => write!(f, "order arena capacity exhausted"),
        }
    }
}

impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::OrderId;

    #[test]
    fn display_is_human_readable() {
        assert_eq!(
            EngineError::UnknownOrder(OrderId(7)).to_string(),
            "unknown order: 7"
        );
    }

    #[test]
    fn error_trait_object_works() {
        let e: Box<dyn std::error::Error> = Box::new(EngineError::CapacityExhausted);
        assert!(e.to_string().contains("capacity"));
    }
}
