use std::{borrow::Cow, fmt::Debug};

mod eq;
mod index;
mod map_primitive;
mod map_reference;
mod normalize;
#[cfg(feature = "parser")]
mod parse;
mod show;
mod stratified;

pub use crate::analysis::{Definitions, TypedDefinitions};
pub use normalize::NormalizationError;
#[cfg(feature = "parser")]
pub use parse::{parse, typed, untyped, ParseError};
use serde::{Deserialize, Serialize};
pub(crate) use show::debug_reference;
pub use show::Show;
pub use stratified::{StratificationError, Stratified};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct Index(pub usize);

#[derive(Serialize, Deserialize, Clone)]
pub enum None {}

impl Show for None {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        panic!()
    }
}

pub trait Primitives<T> {
    fn ty(&self) -> Cow<'_, Term<T, Self>>
    where
        T: Clone,
        Self: Clone;

    fn apply(&self, argument: &Term<T, Self>) -> Term<T, Self>
    where
        Self: Sized;
}

impl<T> Primitives<T> for None {
    fn ty(&self) -> Cow<'_, Term<T>>
    where
        T: Clone,
    {
        panic!()
    }

    fn apply(&self, _: &Term<T>) -> Term<T> {
        panic!()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Term<T, U: Primitives<T> = None> {
    // Untyped language
    Variable(Index),
    Lambda {
        body: Box<Term<T, U>>,
        erased: bool,
    },
    Apply {
        function: Box<Term<T, U>>,
        argument: Box<Term<T, U>>,
        erased: bool,
    },
    Put(Box<Term<T, U>>),
    Duplicate {
        expression: Box<Term<T, U>>,
        body: Box<Term<T, U>>,
    },
    Reference(T),
    Primitive(U),

    // Typed extensions
    Universe,
    Function {
        argument_type: Box<Term<T, U>>,
        return_type: Box<Term<T, U>>,
        erased: bool,
    },
    Annotation {
        checked: bool,
        expression: Box<Term<T, U>>,
        ty: Box<Term<T, U>>,
    },
    Wrap(Box<Term<T, U>>),
}
