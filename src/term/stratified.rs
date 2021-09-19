use derivative::Derivative;

use crate::convert::{NetBuilderExt, NetError};

use std::fmt::Debug;

use super::{
    alloc::{Allocator, Reallocate, System},
    debug_reference,
    normalize::NormalizationError,
    Definitions, Index, None, Primitives, Show, Term,
};

pub struct Stratified<'a, 'b, T, U: Definitions<T, V, A>, V: Primitives<T>, A: Allocator<T, V>>(
    pub(crate) Term<T, V, A>,
    pub(crate) &'a U,
    pub(crate) &'b A,
);

impl<'a, 'b, T: Clone, U: Definitions<T, V, A>, V: Primitives<T> + Clone, A: Allocator<T, V>> Clone
    for Stratified<'a, 'b, T, U, V, A>
{
    fn clone(&self) -> Self {
        Stratified(self.2.copy(&self.0), self.1, self.2)
    }
}

impl<'a, 'b, T, U: Definitions<T, V, A>, V: Primitives<T>, A: Allocator<T, V>>
    Stratified<'a, 'b, T, U, V, A>
{
    pub fn normalize(&mut self) -> Result<(), NormalizationError>
    where
        T: Clone,
        V: Clone,
        A: Reallocate<T, V, A>,
    {
        self.0.normalize_in(self.1, self.2)?;
        Ok(())
    }

    pub fn into_inner(self) -> Term<T, V, A> {
        self.0
    }
}

impl<'a, 'b, T, U: Definitions<T, None, A>, A: Allocator<T, None>>
    Stratified<'a, 'b, T, U, None, A>
{
    pub fn into_net<N: NetBuilderExt<T, U, None, A>>(self) -> Result<N::Net, NetError<T, None, A>> {
        N::build_net(self)
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "T: Show"))]
pub enum StratificationError<T> {
    MultiplicityMismatch,
    AffineUsedInBox,
    DupNonUnitBoxMultiplicity,
    RecursiveDefinition,
    UndefinedReference(#[derivative(Debug(format_with = "debug_reference"))] T),
    ErasedUsed,
}

impl<T, V: Primitives<T>, A: Allocator<T, V>> Term<T, V, A> {
    fn uses(&self) -> usize {
        fn uses_helper<T, V: Primitives<T>, A: Allocator<T, V>>(
            term: &Term<T, V, A>,
            variable: Index,
        ) -> usize {
            use Term::*;
            match term {
                Variable(index) => {
                    if *index == variable {
                        1
                    } else {
                        0
                    }
                }
                Reference(_) | Function { .. } | Universe => 0,
                Lambda { body, erased } => {
                    if *erased {
                        0
                    } else {
                        uses_helper(body, variable.child())
                    }
                }
                Apply {
                    function,
                    argument,
                    erased,
                } => {
                    uses_helper(function, variable)
                        + if *erased {
                            0
                        } else {
                            uses_helper(argument, variable)
                        }
                }
                Put(term) => uses_helper(term, variable),
                Duplicate {
                    expression, body, ..
                } => uses_helper(expression, variable) + uses_helper(body, variable.child()),
                Primitive(_) => todo!(),

                Wrap(term) => uses_helper(term, variable),
                Annotation { expression, ty, .. } => {
                    uses_helper(expression, variable) + uses_helper(ty, variable)
                }
            }
        }

        uses_helper(self, Index::top())
    }

    fn is_boxed_n_times(&self, nestings: usize) -> bool {
        use Term::*;

        fn n_boxes_helper<T, V: Primitives<T>, A: Allocator<T, V>>(
            this: &Term<T, V, A>,
            variable: Index,
            nestings: usize,
            current_nestings: usize,
        ) -> bool {
            match this {
                Reference(_) | Universe | Function { .. } => true,
                Variable(index) => *index != variable || nestings == current_nestings,
                Lambda { body, .. } => {
                    n_boxes_helper(body, variable.child(), nestings, current_nestings)
                }
                Apply {
                    function,
                    argument,
                    erased,
                } => {
                    n_boxes_helper(function, variable, nestings, current_nestings)
                        && (*erased
                            || n_boxes_helper(argument, variable, nestings, current_nestings))
                }
                Put(term) => n_boxes_helper(term, variable, nestings, current_nestings + 1),
                Duplicate { expression, body } => {
                    n_boxes_helper(expression, variable, nestings, current_nestings)
                        && n_boxes_helper(body, variable.child(), nestings, current_nestings)
                }
                Primitive(_) => todo!(),

                Wrap(term) => n_boxes_helper(term, variable, nestings, current_nestings),
                Annotation { expression, .. } => {
                    n_boxes_helper(expression, variable, nestings, current_nestings)
                }
            }
        }

        n_boxes_helper(self, Index::top(), nestings, 0)
    }

    fn is_recursive_in_helper<'a, D: Definitions<T, V, B>, B: Allocator<T, V>>(
        &self,
        seen: &mut Vec<T>,
        definitions: &D,
        alloc: &A,
        b_alloc: &B,
    ) -> bool
    where
        T: PartialEq + Clone,
    {
        use Term::*;

        match self {
            Variable(_) | Universe => false,
            Lambda { body, .. } => body.is_recursive_in_helper(seen, definitions, alloc, b_alloc),
            Apply {
                function,
                argument,
                erased,
            } => {
                (!*erased && argument.is_recursive_in_helper(seen, definitions, alloc, b_alloc))
                    || function.is_recursive_in_helper(seen, definitions, alloc, b_alloc)
            }
            Put(term) => term.is_recursive_in_helper(seen, definitions, alloc, b_alloc),
            Duplicate { expression, body } => {
                expression.is_recursive_in_helper(seen, definitions, alloc, b_alloc)
                    || body.is_recursive_in_helper(seen, definitions, alloc, b_alloc)
            }
            Term::Reference(reference) => {
                if seen.contains(reference) {
                    true
                } else {
                    if let Some(term) = definitions.get(reference) {
                        seen.push(reference.clone());
                        let res = term.as_ref().is_recursive_in_helper(
                            seen,
                            definitions,
                            b_alloc,
                            b_alloc,
                        );
                        seen.pop();
                        res
                    } else {
                        false
                    }
                }
            }
            Term::Primitive(_) => false,
            Term::Function {
                argument_type,
                return_type,
                ..
            } => {
                argument_type.is_recursive_in_helper(seen, definitions, alloc, b_alloc)
                    && return_type.is_recursive_in_helper(seen, definitions, alloc, b_alloc)
            }
            Term::Annotation { expression, .. } => {
                expression.is_recursive_in_helper(seen, definitions, alloc, b_alloc)
            }
            Term::Wrap(term) => term.is_recursive_in_helper(seen, definitions, alloc, b_alloc),
        }
    }

    pub fn is_recursive_in<D: Definitions<T, V, B>, B: Allocator<T, V>>(
        &self,
        definitions: &D,
        alloc: &A,
        b_alloc: &B,
    ) -> bool
    where
        T: PartialEq + Clone,
    {
        self.is_recursive_in_helper(&mut vec![], definitions, alloc, b_alloc)
    }

    pub fn is_sound(&self) -> Result<(), StratificationError<T>>
    where
        T: Clone,
    {
        use Term::*;

        match &self {
            Lambda { body, erased } => {
                if *erased {
                    if body.uses() > 0 {
                        return Err(StratificationError::MultiplicityMismatch);
                    }
                }

                body.is_sound()?;
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                function.is_sound()?;
                if !*erased {
                    argument.is_sound()?;
                }
            }
            Put(term) => {
                term.is_sound()?;
            }
            Duplicate { body, expression } => {
                expression.is_sound()?;
                body.is_sound()?;
            }
            Variable(_) | Reference(_) | Function { .. } | Universe => {}
            Primitive(_) => todo!(),

            Wrap(term) => term.is_sound()?,
            Annotation { expression, .. } => {
                expression.is_sound()?;
            }
        }

        Ok(())
    }

    pub fn is_stratified(&self) -> Result<(), StratificationError<T>>
    where
        T: Clone,
    {
        use Term::*;

        match &self {
            Lambda { body, erased } => {
                if body.uses() > if *erased { 0 } else { 1 } {
                    return Err(StratificationError::MultiplicityMismatch);
                }
                if !body.is_boxed_n_times(0) {
                    return Err(StratificationError::AffineUsedInBox);
                }

                body.is_stratified()?;
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                function.is_stratified()?;
                if !*erased {
                    argument.is_stratified()?;
                }
            }
            Put(term) => {
                term.is_stratified()?;
            }
            Duplicate { body, expression } => {
                if !body.is_boxed_n_times(1) {
                    return Err(StratificationError::DupNonUnitBoxMultiplicity);
                }
                expression.is_stratified()?;
                body.is_stratified()?;
            }
            Variable(_) | Reference(_) | Function { .. } | Universe => {}
            Primitive(_) => todo!(),

            Wrap(term) => term.is_stratified()?,
            Annotation { expression, .. } => {
                expression.is_stratified()?;
            }
        }

        Ok(())
    }

    pub fn stratified_in<'a, 'b, U: Definitions<T, V, A>>(
        self,
        definitions: &'a U,
        allocator: &'b A,
    ) -> Result<Stratified<'a, 'b, T, U, V, A>, StratificationError<T>>
    where
        T: Clone + PartialEq,
    {
        self.is_stratified()?;
        if self.is_recursive_in(definitions, allocator, allocator) {
            Err(StratificationError::RecursiveDefinition)?;
        }
        Ok(Stratified(self, definitions, allocator))
    }
}

static SYSTEM: &'static System = &System;

impl<T: Debug, V: Primitives<T>> Term<T, V, System> {
    pub fn stratified<'a, 'b, U: Definitions<T, V, System>>(
        self,
        definitions: &'a U,
    ) -> Result<Stratified<'a, 'b, T, U, V, System>, StratificationError<T>>
    where
        T: Clone + PartialEq,
    {
        self.stratified_in(definitions, SYSTEM)
    }
}
