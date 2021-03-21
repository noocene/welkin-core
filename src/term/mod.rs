mod normalize;
mod parse;
mod show;
pub use parse::Definitions;

#[derive(Debug, Clone, Copy)]
pub struct Symbol(usize);

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
