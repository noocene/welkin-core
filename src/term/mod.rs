mod normalize;
mod parse;
mod show;
mod stratified;
pub use parse::Definitions;
pub use stratified::{StratificationError, Stratified};

#[derive(Debug, Clone, Copy)]
pub struct Symbol(pub(crate) usize);

#[derive(Clone)]
pub enum Term {
    Symbol(Symbol),
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
}
