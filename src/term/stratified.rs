use std::collections::HashMap;

use super::{normalize::NormalizationError, Term};

#[derive(Debug)]
pub struct Stratified<'a>(Term, &'a HashMap<String, Term>);

impl<'a> Stratified<'a> {
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
        fn uses_helper(term: &Term, depth: usize) -> usize {
            use Term::*;
            match term {
                Symbol(symbol) => {
                    if symbol.0 == depth {
                        1
                    } else {
                        0
                    }
                }
                Reference(_) => 0,
                Lambda { body, .. } => uses_helper(body, depth + 1),
                Apply { function, argument } => {
                    uses_helper(function, depth) + uses_helper(argument, depth)
                }
                Put(term) => uses_helper(term, depth),
                Duplicate {
                    expression, body, ..
                } => uses_helper(expression, depth) + uses_helper(body, depth + 1),
            }
        }

        uses_helper(self, 0)
    }

    fn is_at_level(&self, target_level: usize, depth: usize, level: usize) -> bool {
        use Term::*;

        match self {
            Reference(_) => true,
            Symbol(symbol) => symbol.0 != depth || level == target_level,
            Lambda { body, .. } => body.is_at_level(target_level, depth + 1, level),
            Apply { function, argument } => {
                function.is_at_level(target_level, depth, level)
                    && argument.is_at_level(target_level, depth, level)
            }
            Put(term) => term.is_at_level(target_level, depth, level + 1),
            Duplicate {
                expression, body, ..
            } => {
                expression.is_at_level(target_level, depth, level)
                    && body.is_at_level(target_level, depth + 1, level)
            }
        }
    }

    fn is_stratified(
        &self,
        definitions: &HashMap<String, Term>,
    ) -> Result<(), StratificationError> {
        use Term::*;

        match &self {
            Lambda { body, binding } => {
                if body.uses() > 1 {
                    return Err(StratificationError::AffineReused {
                        name: binding.clone(),
                        term: self.clone(),
                    });
                }
                if !body.is_at_level(0, 0, 0) {
                    return Err(StratificationError::AffineUsedInBox {
                        name: binding.clone(),
                        term: self.clone(),
                    });
                }
                body.is_stratified(definitions)?;
            }
            Apply { function, argument } => {
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
                if !body.is_at_level(1, 0, 0) {
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
            Symbol(_) => {}
        }

        Ok(())
    }

    pub fn stratified(
        self,
        definitions: &HashMap<String, Term>,
    ) -> Result<Stratified<'_>, StratificationError> {
        self.is_stratified(definitions)?;
        Ok(Stratified(self, definitions))
    }
}
