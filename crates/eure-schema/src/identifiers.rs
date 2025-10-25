use eure_value::identifier::Identifier;

// SAFETY: All these strings are valid identifiers according to EURE rules:
// - Start with XID_Start character or underscore
// - Contain only XID_Continue characters or hyphens
// - Are not reserved keywords (true, false, null)
// - Do not start with $

pub const SCHEMA: Identifier = unsafe { Identifier::new_unchecked("schema") };
pub const CASCADE_TYPE: Identifier = unsafe { Identifier::new_unchecked("cascade-type") };
pub const RENAME: Identifier = unsafe { Identifier::new_unchecked("rename") };
pub const RENAME_ALL: Identifier = unsafe { Identifier::new_unchecked("rename-all") };
pub const VARIANTS: Identifier = unsafe { Identifier::new_unchecked("variants") };
pub const VARIANT_REPR: Identifier = unsafe { Identifier::new_unchecked("variant-repr") };
pub const VARIANT: Identifier = unsafe { Identifier::new_unchecked("variant") };
pub const ARRAY: Identifier = unsafe { Identifier::new_unchecked("array") };
pub const UNKNOWN: Identifier = unsafe { Identifier::new_unchecked("unknown") };
pub const UNKNOWN_CAPS: Identifier = unsafe { Identifier::new_unchecked("Unknown") };
pub const TAG: Identifier = unsafe { Identifier::new_unchecked("tag") };
pub const CONTENT: Identifier = unsafe { Identifier::new_unchecked("content") };
