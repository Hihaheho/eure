use eure_document::document::NodeId;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum EureToJsonError {
    #[error("Hole (uninitialized value) is not supported in JSON")]
    HoleNotSupported { node_id: NodeId },

    #[error("PartialMap (map with hole keys) is not supported in JSON")]
    PartialMapNotSupported { node_id: NodeId },

    #[error("BigInt value is out of range for JSON number")]
    BigIntOutOfRange { node_id: NodeId },

    #[error("Non-finite floating point value (NaN or Infinity) is not supported in JSON")]
    NonFiniteFloat { node_id: NodeId },

    #[error("Decimal value cannot be represented as a JSON number without losing precision")]
    DecimalPrecisionLoss { node_id: NodeId },

    #[error("Variant content already contains tag field '{tag}' in Internal representation")]
    VariantTagConflict { tag: String, node_id: NodeId },

    #[error("Variant content already contains field '{field}' in Adjacent representation")]
    VariantAdjacentConflict { field: String, node_id: NodeId },
}

impl EureToJsonError {
    /// Returns the NodeId associated with this error.
    pub fn node_id(&self) -> NodeId {
        match self {
            EureToJsonError::HoleNotSupported { node_id } => *node_id,
            EureToJsonError::PartialMapNotSupported { node_id } => *node_id,
            EureToJsonError::BigIntOutOfRange { node_id } => *node_id,
            EureToJsonError::NonFiniteFloat { node_id } => *node_id,
            EureToJsonError::DecimalPrecisionLoss { node_id } => *node_id,
            EureToJsonError::VariantTagConflict { node_id, .. } => *node_id,
            EureToJsonError::VariantAdjacentConflict { node_id, .. } => *node_id,
        }
    }
}

/// Errors that can occur when converting JSON to Eure.
#[derive(Debug, Error, PartialEq)]
pub enum JsonToEureError {
    #[error("JSON number {number} cannot be represented as an Eure value")]
    UnrepresentableNumber { number: String },
}
