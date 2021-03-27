mod eq;
mod index;
mod normalize;
mod parse;
mod show;
mod stratified;

pub use crate::analysis::{Definitions, TypedDefinitions};
pub use normalize::NormalizationError;
pub use parse::{typed, untyped, ParseError};
pub(crate) use show::debug_reference;
pub use show::Show;
pub use stratified::{StratificationError, Stratified};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Index(pub(crate) usize);

#[derive(Clone)]
pub enum Term<T> {
    // Untyped language
    Variable(Index),
    Lambda {
        body: Box<Term<T>>,
        erased: bool,
    },
    Apply {
        function: Box<Term<T>>,
        argument: Box<Term<T>>,
        erased: bool,
    },
    Put(Box<Term<T>>),
    Duplicate {
        expression: Box<Term<T>>,
        body: Box<Term<T>>,
    },
    Reference(T),

    // Typed extensions
    Universe,
    Function {
        argument_type: Box<Term<T>>,
        return_type: Box<Term<T>>,
        erased: bool,
    },
    Annotation {
        checked: bool,
        expression: Box<Term<T>>,
        ty: Box<Term<T>>,
    },
    Wrap(Box<Term<T>>),
}
