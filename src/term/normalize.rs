use std::collections::HashMap;

use super::Term;

#[derive(Debug)]
pub enum NormalizationError {
    InvalidDuplication,
    InvalidApplication,
}

impl Term {
    fn shift(&mut self, increment: usize, depth: usize) {
        use Term::*;

        match self {
            Symbol(symbol) => {
                if !(symbol.0 < depth) {
                    symbol.0 += increment;
                }
            }
            Lambda { body, .. } => body.shift(increment, depth + 1),
            Apply { function, argument } => {
                function.shift(increment, depth);
                argument.shift(increment, depth);
            }
            Put(term) => {
                term.shift(increment, depth);
            }
            Duplicate {
                expression, body, ..
            } => {
                expression.shift(increment, depth);
                body.shift(increment, depth + 1);
            }
            Reference(_) => {}
        }
    }

    fn substitute(&mut self, value: &Term, depth: usize) {
        use Term::*;

        match self {
            Symbol(symbol) => {
                if depth == symbol.0 {
                    *self = value.clone();
                } else if symbol.0 > depth {
                    symbol.0 -= 1;
                }
            }
            Lambda { body, .. } => {
                let mut value = value.clone();
                value.shift(1, 0);
                body.substitute(&value, depth + 1);
            }
            Apply { function, argument } => {
                function.substitute(value, depth);
                argument.substitute(value, depth);
            }
            Put(term) => {
                term.substitute(value, depth);
            }
            Duplicate {
                body, expression, ..
            } => {
                expression.substitute(value, depth);
                let mut value = value.clone();
                value.shift(1, 0);
                body.substitute(&value, depth + 1);
            }
            Reference(_) => {}
        }
    }

    pub(crate) fn normalize(
        &mut self,
        definitions: &HashMap<String, Term>,
    ) -> Result<(), NormalizationError> {
        use Term::*;

        match self {
            Reference(binding) => {
                if let Some(term) = definitions.get(binding).map(|term| {
                    let mut term = term.clone();
                    term.normalize(definitions)?;
                    Ok(term)
                }) {
                    *self = term?;
                }
            }
            Lambda { body, .. } => {
                body.normalize(definitions)?;
            }
            Put(term) => {
                term.normalize(definitions)?;
            }
            Duplicate {
                body,
                expression,
                binding,
            } => {
                expression.normalize(definitions)?;
                match &**expression {
                    Put(expression) => {
                        body.substitute(&expression, 0);
                        body.normalize(definitions)?;
                        *self = *body.clone();
                    }
                    Duplicate {
                        binding: new_binding,
                        expression,
                        body: new_body,
                    } => {
                        body.shift(1, 1);
                        let binding = binding.clone();
                        let dup = Duplicate {
                            body: body.clone(),
                            expression: new_body.clone(),
                            binding,
                        };
                        let mut term = Duplicate {
                            binding: new_binding.clone(),
                            expression: expression.clone(),
                            body: Box::new(dup),
                        };
                        term.normalize(definitions)?;
                        *self = term;
                    }
                    Lambda { .. } => Err(NormalizationError::InvalidDuplication)?,
                    _ => {
                        body.normalize(definitions)?;
                    }
                }
            }
            Apply { function, argument } => {
                function.normalize(definitions)?;
                let function = function.clone();
                match *function {
                    Put(_) => Err(NormalizationError::InvalidApplication)?,
                    Duplicate {
                        body,
                        expression,
                        binding,
                    } => {
                        let mut argument = argument.clone();
                        argument.shift(1, 0);
                        let body = Box::new(Apply {
                            function: body,
                            argument,
                        });
                        let mut term = Duplicate {
                            binding,
                            expression,
                            body,
                        };
                        term.normalize(definitions)?;
                        *self = term;
                    }
                    Lambda { mut body, .. } => {
                        body.substitute(argument, 0);
                        body.normalize(definitions)?;
                        *self = *body;
                    }
                    _ => {
                        argument.normalize(definitions)?;
                    }
                }
            }
            Symbol(_) => {}
        }

        Ok(())
    }
}
