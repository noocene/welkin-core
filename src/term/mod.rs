mod parse;
mod show;
pub use parse::Definitions;
pub use show::Contextualized;

#[derive(Debug, Clone, Copy)]
pub struct Symbol(usize);

#[derive(Debug, Clone)]
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
    Box(Box<Term>),
    Duplicate {
        name: String,
        expression: Box<Term>,
        body: Box<Term>,
    },
    Reference(String),
}
