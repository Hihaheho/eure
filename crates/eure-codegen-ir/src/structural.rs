use crate::error::StructuralDiff;
use crate::module::IrModule;

pub fn structural_eq(lhs: &IrModule, rhs: &IrModule) -> bool {
    lhs == rhs
}

pub fn assert_structural_eq(lhs: &IrModule, rhs: &IrModule) -> Result<(), StructuralDiff> {
    if lhs == rhs {
        Ok(())
    } else {
        Err(StructuralDiff::new(
            "module",
            format!(
                "structural modules differ\nleft={:#?}\nright={:#?}",
                lhs, rhs
            ),
        ))
    }
}
