#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QualifiedTypeName {
    pub namespace: Option<String>,
    pub name: String,
}

impl QualifiedTypeName {
    pub fn new(namespace: Option<String>, name: impl Into<String>) -> Self {
        Self {
            namespace,
            name: name.into(),
        }
    }

    pub fn local(name: impl Into<String>) -> Self {
        Self {
            namespace: None,
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RustPathIr {
    pub path: String,
}

impl RustPathIr {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SchemaNodeIrId(pub usize);
