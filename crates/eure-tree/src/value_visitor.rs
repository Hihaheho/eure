#![expect(unused)]
use ahash::AHashMap;
use eure_value::{identifier::Identifier, value::PathSegment};
use thiserror::Error;

use crate::{prelude::*, tree::CstFacade};

pub struct Values {
    ident_handles: AHashMap<IdentHandle, Identifier>,
    key_handles: AHashMap<KeyHandle, PathSegment>,
    keys_handles: AHashMap<KeysHandle, Vec<KeyHandle>>,
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    values: &'a mut Values,
    current_keys: Vec<KeyHandle>,
}

#[derive(Debug, Error)]
pub enum ValueVisitorError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
}

impl<F: CstFacade> CstVisitor<F> for ValueVisitor<'_> {
    type Error = ValueVisitorError;

    fn visit_keys(
        &mut self,
        handle: KeysHandle,
        view: KeysView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        assert_eq!(self.current_keys.len(), 0);
        self.visit_keys_super(handle, view, tree)?;
        self.values
            .keys_handles
            .insert(handle, std::mem::take(&mut self.current_keys));
        Ok(())
    }

    fn visit_key(&mut self, handle: KeyHandle, view: KeyView, tree: &F) -> Result<(), Self::Error> {
        self.visit_key_super(handle, view, tree)?;
        Ok(())
    }

    fn visit_key_base(
        &mut self,
        handle: KeyBaseHandle,
        view: KeyBaseView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_base_super(handle, view, tree)?;
        Ok(())
    }

    fn visit_ident(
        &mut self,
        handle: IdentHandle,
        view: IdentView,
        tree: &F,
    ) -> Result<(), Self::Error> {
        let data = view.ident.get_data(tree)?;
        let text = tree.get_str(data, self.input).unwrap();
        self.values.ident_handles.insert(handle, todo!());
        Ok(())
    }

    fn visit_key_opt(
        &mut self,
        handle: KeyOptHandle,
        view: ArrayMarkerHandle,
        tree: &F,
    ) -> Result<(), Self::Error> {
        self.visit_key_opt_super(handle, view, tree)?;
        Ok(())
    }
}
