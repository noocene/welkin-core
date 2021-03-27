use super::{normalize::NormalizationError, Definitions, Index, Term};

#[derive(Debug, Clone)]
pub struct Stratified<'a, U: Definitions>(pub(crate) Term, pub(crate) &'a U);

impl<'a, U: Definitions> Stratified<'a, U> {
    pub fn normalize(&mut self) -> Result<(), NormalizationError> {
        self.0.normalize(self.1)
    }

    pub fn into_inner(self) -> Term {
        self.0
    }
}

#[derive(Debug)]
pub enum StratificationError {
    AffineReused { name: String, term: Term },
    AffineUsedInBox { name: String, term: Term },
    DupNonUnitBoxMultiplicity { name: String, term: Term },
    UndefinedReference { name: String },
}

impl Term {
    fn uses(&self) -> usize {
        fn uses_helper(term: &Term, variable: Index) -> usize {
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

        fn n_boxes_helper(
            this: &Term,
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
                Duplicate {
                    expression, body, ..
                } => {
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

    pub fn is_stratified<U: Definitions>(
        &self,
        definitions: &U,
    ) -> Result<(), StratificationError> {
        use Term::*;

        match &self {
            Lambda { body, binding, .. } => {
                if body.uses() > 1 {
                    return Err(StratificationError::AffineReused {
                        name: binding.clone(),
                        term: self.clone(),
                    });
                }
                if !body.is_boxed_n_times(0) {
                    return Err(StratificationError::AffineUsedInBox {
                        name: binding.clone(),
                        term: self.clone(),
                    });
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
            Duplicate {
                binding,
                body,
                expression,
            } => {
                if !body.is_boxed_n_times(1) {
                    return Err(StratificationError::DupNonUnitBoxMultiplicity {
                        name: binding.clone(),
                        term: self.clone(),
                    });
                }
                expression.is_stratified(definitions)?;
                body.is_stratified(definitions)?;
            }
            Reference(name) => {
                if let Some(term) = definitions.get(name) {
                    term.is_stratified(definitions)?;
                } else {
                    return Err(StratificationError::UndefinedReference { name: name.clone() });
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
                ..
            } => {
                if !erased {
                    argument_type.is_stratified(definitions)?;
                    return_type.is_stratified(definitions)?;
                }
            }
        }

        Ok(())
    }

    pub fn stratified<U: Definitions>(
        self,
        definitions: &U,
    ) -> Result<Stratified<'_, U>, StratificationError> {
        self.is_stratified(definitions)?;
        Ok(Stratified(self, definitions))
    }
}
