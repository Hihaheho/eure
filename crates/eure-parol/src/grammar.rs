use crate::grammar_trait::{Eure, EureRoot, GrammarTrait, Root};
#[allow(unused_imports)]
use parol_runtime::{Result, Token};
use std::fmt::{Debug, Display, Error, Formatter};

///
/// Data structure that implements the semantic actions for our Eure grammar
/// !Change this type as needed!
///
#[derive(Debug, Default)]
pub struct Grammar<'t> {
    pub eure_root: Option<EureRoot<'t>>,
}

impl Grammar<'_> {
    pub fn new() -> Self {
        Grammar::default()
    }
}

impl Display for EureRoot<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), Error> {
        write!(f, "{self:?}")
    }
}

impl Display for Grammar<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), Error> {
        match &self.eure_root {
            Some(eure_root) => writeln!(f, "{eure_root}"),
            None => write!(f, "No parse result"),
        }
    }
}

impl<'t> GrammarTrait<'t> for Grammar<'t> {
    /// Semantic action for non-terminal 'Eure'
    fn eure_root(&mut self, arg: &EureRoot<'t>) -> Result<()> {
        self.root = Some(arg.clone());
        Ok(())
    }
}
