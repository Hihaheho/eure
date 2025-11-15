use crate::{
    document::DocumentKey,
    value::{EurePath, PrimitiveValue},
};

/// No ambiguous commands for initializing or manipulating the document.
pub enum DocumentCommand {
    /// Move the cursor to the root of the document.
    MoveToRoot,
    /// Ensure that the path points to an object or creates it recursively if it doesn't exist. Error if the path points to a non-object.
    RecursivelyEnsureObject { path: EurePath },
    /// Bind a value to a key.
    Bind {
        key: DocumentKey,
        value: PrimitiveValue,
    },
}
