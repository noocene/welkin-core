use std::convert::Infallible;

use super::{
    alloc::{Allocator, IntoInner, Zero},
    Primitives, Term,
};

impl<T, V: Primitives<T>, A: Allocator<T, V>> Term<T, V, A> {
    pub fn try_map_reference_in<U, E, F: Fn(T) -> Result<Term<U, V, A>, E> + Clone>(
        self,
        f: F,
        alloc: &A,
    ) -> Result<Term<U, V, A>, E>
    where
        V: Primitives<U>,
        A: Allocator<U, V>,
    {
        use Term::*;

        Ok(match self {
            Variable(var) => Variable(var),
            Lambda { body, erased } => Lambda {
                body: alloc.alloc(body.into_inner().try_map_reference_in(f, alloc)?),
                erased,
            },
            Primitive(primitive) => Primitive(primitive),
            Apply {
                function,
                argument,
                erased,
            } => Apply {
                function: alloc.alloc(
                    function
                        .into_inner()
                        .try_map_reference_in(f.clone(), alloc)?,
                ),
                argument: alloc.alloc(argument.into_inner().try_map_reference_in(f, alloc)?),
                erased,
            },
            Put(term) => Put(alloc.alloc(term.into_inner().try_map_reference_in(f, alloc)?)),
            Duplicate { expression, body } => Duplicate {
                expression: alloc.alloc(
                    expression
                        .into_inner()
                        .try_map_reference_in(f.clone(), alloc)?,
                ),
                body: alloc.alloc(body.into_inner().try_map_reference_in(f, alloc)?),
            },
            Reference(reference) => f(reference)?,
            Universe => Universe,
            Function {
                argument_type,
                return_type,
                erased,
            } => Function {
                argument_type: alloc.alloc(
                    argument_type
                        .into_inner()
                        .try_map_reference_in(f.clone(), alloc)?,
                ),
                return_type: alloc.alloc(return_type.into_inner().try_map_reference_in(f, alloc)?),
                erased,
            },
            Annotation {
                checked,
                expression,
                ty,
            } => Annotation {
                expression: alloc.alloc(
                    expression
                        .into_inner()
                        .try_map_reference_in(f.clone(), alloc)?,
                ),
                ty: alloc.alloc(ty.into_inner().try_map_reference_in(f, alloc)?),
                checked,
            },
            Wrap(term) => Wrap(alloc.alloc(term.into_inner().try_map_reference_in(f, alloc)?)),
        })
    }

    pub fn map_reference_in<U, F: Clone + Fn(T) -> Term<U, V, A>>(
        self,
        f: F,
        alloc: &A,
    ) -> Term<U, V, A>
    where
        V: Primitives<U>,
        A: Allocator<U, V>,
    {
        self.try_map_reference_in(|a| Ok::<_, Infallible>(f(a)), alloc)
            .unwrap()
    }

    pub fn try_map_reference<U, E, F: Fn(T) -> Result<Term<U, V, A>, E> + Clone>(
        self,
        f: F,
    ) -> Result<Term<U, V, A>, E>
    where
        V: Primitives<U>,
        A: Allocator<U, V> + Zero,
    {
        let alloc = A::zero();

        self.try_map_reference_in(f, &alloc)
    }

    pub fn map_reference<U, F: Clone + Fn(T) -> Term<U, V, A>>(self, f: F) -> Term<U, V, A>
    where
        V: Primitives<U>,
        A: Allocator<U, V> + Zero,
    {
        let alloc = A::zero();

        self.map_reference_in(f, &alloc)
    }
}
