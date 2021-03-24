mod index;
mod normalize;
mod parse;
mod show;
mod stratified;
use std::collections::HashMap;

pub use parse::{typed, untyped};
pub use stratified::{StratificationError, Stratified};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Index(pub(crate) usize);

#[derive(Clone)]
pub enum Term {
    // Untyped language
    Variable(Index),
    Lambda {
        binding: String,
        body: Box<Term>,
    },
    Apply {
        function: Box<Term>,
        argument: Box<Term>,
    },
    Put(Box<Term>),
    Duplicate {
        binding: String,
        expression: Box<Term>,
        body: Box<Term>,
    },
    Reference(String),

    // Typed extensions,
    Universe,
    Function {
        self_binding: String,
        argument_binding: String,
        argument_type: Box<Term>,
        return_type: Box<Term>,
    },
    Annotation {
        checked: bool,
        expression: Box<Term>,
        ty: Box<Term>,
    },
    Wrap(Box<Term>),
}

mod sealed {
    use std::collections::HashMap;

    use super::Term;

    pub trait SealedDefinitions {}

    impl SealedDefinitions for HashMap<String, Term> {}
    impl SealedDefinitions for HashMap<String, (Term, Term)> {}
}

pub trait Definitions: sealed::SealedDefinitions {
    fn get(&self, name: &str) -> Option<&Term>;
}

impl Definitions for HashMap<String, Term> {
    fn get(&self, name: &str) -> Option<&Term> {
        HashMap::get(self, name)
    }
}

impl Definitions for HashMap<String, (Term, Term)> {
    fn get(&self, name: &str) -> Option<&Term> {
        HashMap::get(self, name).map(|(_, b)| b)
    }
}
