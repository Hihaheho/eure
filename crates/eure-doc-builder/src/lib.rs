mod migrate;
mod site;

pub use migrate::*;
pub use site::*;

pub const NAV_SCHEMA: &str = include_str!("../assets/nav.schema.eure");
pub const ADR_SCHEMA: &str = include_str!("../assets/adr.schema.eure");
