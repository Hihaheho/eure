pub mod document;
pub mod error;
pub mod report;
pub mod tree;
pub mod value;

pub use eure_document::data_model;
pub use eure_document::eure;
pub use eure_document::parse::ParseDocument;
pub use eure_macros::ParseDocument;
pub use eure_parol as parol;
