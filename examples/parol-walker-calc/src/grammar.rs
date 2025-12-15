use crate::grammar_trait::{Calc, GrammarTrait};
#[allow(unused_imports)]
use parol_runtime::{Result, Token};
use std::fmt::{Debug, Display, Error, Formatter};

/// Data structure that implements the semantic actions for our Calc grammar
#[derive(Debug, Default)]
pub struct Grammar<'t> {
    pub calc: Option<Calc<'t>>,
}

impl Grammar<'_> {
    pub fn new() -> Self {
        Grammar::default()
    }
}

impl Display for Calc<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), Error> {
        write!(f, "{self:?}")
    }
}

impl Display for Grammar<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), Error> {
        match &self.calc {
            Some(calc_root) => writeln!(f, "{calc_root}"),
            None => write!(f, "No parse result"),
        }
    }
}

impl<'t> GrammarTrait<'t> for Grammar<'t> {
    /// Semantic action for non-terminal 'Calc'
    fn calc(&mut self, arg: &Calc<'t>) -> Result<()> {
        self.calc = Some(arg.clone());
        Ok(())
    }
}
