use std::fmt::Debug;

pub mod alloc;
use alloc::{Allocator, IntoInner, System, Zero};
mod eq;
pub use eq::{EqualityCache, MapCache, NullCache};
mod hash;
mod index;
mod map_primitive;
mod map_reference;
mod normalize;
#[cfg(feature = "parser")]
mod parse;
mod serde_impls;
mod show;
mod stratified;

pub use crate::analysis::{AnalysisError, DefinitionResult, Definitions, TypedDefinitions};
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum None {}

impl Show for None {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        panic!()
    }
}

pub trait Primitives<T>: Sized {
    fn ty<A: Allocator<T, Self>>(&self, alloc: &A) -> Term<T, Self, A>;

    fn apply<A: Allocator<T, Self>>(
        &self,
        argument: &Term<T, Self, A>,
        alloc: &A,
    ) -> Term<T, Self, A>
    where
        Self: Sized;
}

impl<T> Primitives<T> for None {
    fn ty<A: Allocator<T, Self>>(&self, _: &A) -> Term<T, Self, A> {
        panic!()
    }

    fn apply<A: Allocator<T, Self>>(&self, _: &Term<T, Self, A>, _: &A) -> Term<T, Self, A> {
        panic!()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize, U: Serialize",
    deserialize = "T: Deserialize<'de>, U: Deserialize<'de>, A: Zero"
))]
pub enum Term<T, U: Primitives<T> = None, A: Allocator<T, U> = System> {
    // Untyped language
    Variable(Index),
    Lambda {
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        body: A::Box,
        erased: bool,
    },
    Apply {
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        function: A::Box,
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        argument: A::Box,
        erased: bool,
    },
    Put(
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        A::Box,
    ),
    Duplicate {
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        expression: A::Box,
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        body: A::Box,
    },
    Reference(T),
    Primitive(U),

    // Typed extensions
    Universe,
    Function {
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        argument_type: A::Box,
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        return_type: A::Box,
        erased: bool,
    },
    Annotation {
        checked: bool,
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        expression: A::Box,
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        ty: A::Box,
    },
    Wrap(
        #[serde(
            serialize_with = "serde_impls::serialize::<T, U, A, _>",
            deserialize_with = "serde_impls::deserialize::<T, U, A, _>"
        )]
        A::Box,
    ),
}
