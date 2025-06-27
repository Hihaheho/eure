//! Hand-written convenience layer for CST construction
//! 
//! This module provides ergonomic APIs for constructing CST nodes,
//! wrapping the generated constructors with more intuitive interfaces.

use crate::generated::constructors::{self, constructors::*};
use crate::nodes::*;
use crate::tree::CstFacade;

/// Convenience constructor for creating an array with elements
pub fn array<F: CstFacade>(
    tree: &mut F,
    elements: impl IntoIterator<Item = ValueHandle>,
) -> ArrayHandle {
    // TODO: Implement using generated constructors
    unimplemented!("Array convenience constructor not yet implemented")
}

/// Convenience constructor for creating an object with fields
pub fn object<F: CstFacade>(
    tree: &mut F,
    fields: impl IntoIterator<Item = (KeyHandle, ValueHandle)>,
) -> ObjectHandle {
    // TODO: Implement using generated constructors
    unimplemented!("Object convenience constructor not yet implemented")
}

/// Convenience constructor for creating an integer value
pub fn integer<F: CstFacade>(tree: &mut F, value: i64) -> ValueHandle {
    let int_str = value.to_string();
    let int_handle = terminals::integer(tree, &int_str);
    let int_constructor = IntegerConstructor::builder()
        .child_0(int_handle)
        .build();
    let integer_handle = int_constructor.build_with_tree(tree);
    
    // Wrap in Value
    ValueConstructor::Integer(integer_handle).build_with_tree(tree)
}

/// Convenience constructor for creating a string value
pub fn string<F: CstFacade>(tree: &mut F, value: &str) -> ValueHandle {
    let str_handle = terminals::str(tree, value);
    let str_constructor = StrConstructor::builder()
        .child_0(str_handle)
        .build();
    let str_handle = str_constructor.build_with_tree(tree);
    
    // Create Strings (single string without continuation)
    let strings_list = StringsListConstructor::builder()
        .build()
        .build_with_tree(tree);
    let strings_constructor = StringsConstructor::builder()
        .child_0(str_handle)
        .child_1(strings_list)
        .build();
    let strings_handle = strings_constructor.build_with_tree(tree);
    
    // Wrap in Value
    ValueConstructor::Strings(strings_handle).build_with_tree(tree)
}

/// Convenience constructor for creating a boolean value
pub fn boolean<F: CstFacade>(tree: &mut F, value: bool) -> ValueHandle {
    let bool_handle = if value {
        let true_terminal = terminals::r#true(tree);
        let true_constructor = TrueConstructor::builder()
            .child_0(true_terminal)
            .build();
        let true_handle = true_constructor.build_with_tree(tree);
        BooleanConstructor::True(true_handle).build_with_tree(tree)
    } else {
        let false_terminal = terminals::r#false(tree);
        let false_constructor = FalseConstructor::builder()
            .child_0(false_terminal)
            .build();
        let false_handle = false_constructor.build_with_tree(tree);
        BooleanConstructor::False(false_handle).build_with_tree(tree)
    };
    
    // Wrap in Value
    ValueConstructor::Boolean(bool_handle).build_with_tree(tree)
}

/// Convenience constructor for creating a null value
pub fn null<F: CstFacade>(tree: &mut F) -> ValueHandle {
    let null_terminal = terminals::null(tree);
    let null_constructor = NullConstructor::builder()
        .child_0(null_terminal)
        .build();
    let null_handle = null_constructor.build_with_tree(tree);
    
    // Wrap in Value
    ValueConstructor::Null(null_handle).build_with_tree(tree)
}

/// Builder for constructing sections with bindings
pub struct SectionBuilder<F> {
    tree: F,
    name: String,
    bindings: Vec<BindingHandle>,
}

impl<F: CstFacade> SectionBuilder<F> {
    pub fn new(tree: F, name: impl Into<String>) -> Self {
        Self {
            tree,
            name: name.into(),
            bindings: Vec::new(),
        }
    }
    
    pub fn add_binding(mut self, key: &str, value: ValueHandle) -> Self {
        // TODO: Create binding from key and value
        unimplemented!("SectionBuilder::add_binding not yet implemented")
    }
    
    pub fn build(self) -> SectionHandle {
        // TODO: Build section using generated constructors
        unimplemented!("SectionBuilder::build not yet implemented")
    }
}

/// Helper to create a key from a string
pub fn key<F: CstFacade>(tree: &mut F, name: &str) -> KeyHandle {
    let ident = terminals::ident(tree, name);
    let ident_constructor = IdentConstructor::builder()
        .child_0(ident)
        .build();
    let ident_handle = ident_constructor.build_with_tree(tree);
    
    // Create KeyBase from Ident
    let key_base = KeyBaseConstructor::Ident(ident_handle).build_with_tree(tree);
    
    // Create Key without array marker
    let key_opt = KeyOptConstructor::builder()
        .value(None)
        .build()
        .build_with_tree(tree);
    
    KeyConstructor::builder()
        .child_0(key_base)
        .child_1(key_opt)
        .build()
        .build_with_tree(tree)
}

/// Helper to create a simple binding
pub fn binding<F: CstFacade>(tree: &mut F, key_name: &str, value: ValueHandle) -> BindingHandle {
    let key_handle = key(tree, key_name);
    
    // Create Keys from single Key
    let keys_list = KeysListConstructor::builder()
        .build()
        .build_with_tree(tree);
    let keys = KeysConstructor::builder()
        .child_0(key_handle)
        .child_1(keys_list)
        .build()
        .build_with_tree(tree);
    
    // Create Bind terminal
    let bind = terminals::bind(tree);
    let bind_constructor = BindConstructor::builder()
        .child_0(bind)
        .build();
    let bind_handle = bind_constructor.build_with_tree(tree);
    
    // Create ValueBinding
    let value_binding = ValueBindingConstructor::builder()
        .child_0(bind_handle)
        .child_1(value)
        .build()
        .build_with_tree(tree);
    
    // Wrap in BindingRhs
    let binding_rhs = BindingRhsConstructor::ValueBinding(value_binding).build_with_tree(tree);
    
    // Create Binding
    BindingConstructor::builder()
        .child_0(keys)
        .child_1(binding_rhs)
        .build()
        .build_with_tree(tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore = "Constructors not yet fully implemented"]
    fn test_value_constructors() {
        // TODO: Add tests once constructors are implemented
    }
}