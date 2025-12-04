use crate::path::PathSegment;

/// A path segment with optional origin information.
///
/// Used when constructing documents to track where each segment came from.
#[derive(Debug, Clone)]
pub struct Segment<O> {
    pub path: PathSegment,
    pub origin: Option<O>,
}

impl<O> Segment<O> {
    /// Create a segment without origin information.
    pub fn new(path: PathSegment) -> Self {
        Self { path, origin: None }
    }

    /// Create a segment with origin information.
    pub fn with_origin(path: PathSegment, origin: O) -> Self {
        Self {
            path,
            origin: Some(origin),
        }
    }
}

impl<O> From<PathSegment> for Segment<O> {
    fn from(path: PathSegment) -> Self {
        Self::new(path)
    }
}
