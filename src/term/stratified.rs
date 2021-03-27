use derivative::Derivative;

use super::{debug_reference, normalize::NormalizationError, Definitions, Index, Show, Term};

#[derive(Clone)]
pub struct Stratified<'a, T, U: Definitions<T>>(pub(crate) Term<T>, pub(crate) &'a U);

impl<'a, T, U: Definitions<T>> Stratified<'a, T, U> {
    pub fn normalize(&mut self) -> Result<(), NormalizationError>
    where
        T: Clone,
    {
        self.0.normalize(self.1)
    }

    pub fn into_inner(self) -> Term<T> {
        self.0
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = "T: Show"))]
pub enum StratificationError<T> {
    AffineReused(Term<T>),
    AffineUsedInBox(Term<T>),
    DupNonUnitBoxMultiplicity(Term<T>),
    UndefinedReference(#[derivative(Debug(format_with = "debug_reference"))] T),
}

impl<T> Term<T> {
    fn uses(&self) -> usize {
        fn uses_helper<T>(term: &Term<T>, variable: Index) -> usize {
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
                Lambda { body, .. } => uses_helper(body, variable.child()),
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

        fn n_boxes_helper<T>(
            this: &Term<T>,
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

                Wrap(term) => n_boxes_helper(term, variable, nestings, current_nestings),
                Annotation { expression, .. } => {
                    n_boxes_helper(expression, variable, nestings, current_nestings)
                }
            }
        }

        n_boxes_helper(self, Index::top(), nestings, 0)
    }

    pub fn is_stratified<U: Definitions<T>>(
        &self,
        definitions: &U,
    ) -> Result<(), StratificationError<T>>
    where
        T: Clone,
    {
        use Term::*;

        match &self {
            Lambda { body, .. } => {
                if body.uses() > 1 {
                    return Err(StratificationError::AffineReused(self.clone()));
                }
                if !body.is_boxed_n_times(0) {
                    return Err(StratificationError::AffineUsedInBox(self.clone()));
                }

                body.is_stratified(definitions)?;
            }
            Apply {
                function, argument, ..
            } => {
                function.is_stratified(definitions)?;
                argument.is_stratified(definitions)?;
            }
            Put(term) => {
                term.is_stratified(definitions)?;
            }
            Duplicate { body, expression } => {
                if !body.is_boxed_n_times(1) {
                    return Err(StratificationError::DupNonUnitBoxMultiplicity(self.clone()));
                }
                expression.is_stratified(definitions)?;
                body.is_stratified(definitions)?;
            }
            Reference(reference) => {
                if let Some(term) = definitions.get(reference) {
                    term.is_stratified(definitions)?;
                } else {
                    return Err(StratificationError::UndefinedReference(reference.clone()));
                }
            }
            Variable(_) | Universe => {}

            Wrap(term) => term.is_stratified(definitions)?,
            Annotation { expression, ty, .. } => {
                expression.is_stratified(definitions)?;
                ty.is_stratified(definitions)?;
            }
            Function {
                argument_type,
                return_type,
                erased,
            } => {
                if !erased {
                    argument_type.is_stratified(definitions)?;
                    return_type.is_stratified(definitions)?;
                }
            }
        }

        Ok(())
    }

    pub fn stratified<U: Definitions<T>>(
        self,
        definitions: &U,
    ) -> Result<Stratified<'_, T, U>, StratificationError<T>>
    where
        T: Clone,
    {
        self.is_stratified(definitions)?;
        Ok(Stratified(self, definitions))
    }
}
