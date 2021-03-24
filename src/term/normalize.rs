use super::{Definitions, Index, Term};

#[derive(Debug)]
pub enum NormalizationError {
    InvalidDuplication,
    InvalidApplication,
}

impl Term {
    fn shift(&mut self, replaced: Index) {
        use Term::*;

        match self {
            Variable(index) => {
                if index.within(replaced) || *index == replaced {
                    *index = index.child();
                }
            }
            Lambda { body, .. } => body.shift(replaced.child()),
            Apply { function, argument } => {
                function.shift(replaced);
                argument.shift(replaced);
            }
            Put(term) => {
                term.shift(replaced);
            }
            Duplicate {
                expression, body, ..
            } => {
                expression.shift(replaced);
                body.shift(replaced.child());
            }
            Reference(_) => {}
            _ => todo!("handle typed terms"),
        }
    }

    fn shift_top(&mut self) {
        self.shift(Index::top())
    }

    fn substitute_shifted(&mut self, variable: Index, term: &Term) {
        let mut term = term.clone();
        term.shift_top();
        self.substitute(variable.child(), &term)
    }

    fn substitute(&mut self, variable: Index, term: &Term) {
        use Term::*;

        match self {
            Variable(idx) => {
                if variable == *idx {
                    *self = term.clone();
                } else if idx.within(variable) {
                    *idx = idx.parent();
                }
            }
            Lambda { body, .. } => {
                body.substitute_shifted(variable, term);
            }
            Apply { function, argument } => {
                function.substitute(variable, term);
                argument.substitute(variable, term);
            }
            Put(expr) => {
                expr.substitute(variable, term);
            }
            Duplicate {
                body, expression, ..
            } => {
                expression.substitute(variable, term);
                body.substitute_shifted(variable, term);
            }
            Reference(_) => {}
            _ => todo!("handle typed terms"),
        }
    }

    fn substitute_top(&mut self, term: &Term) {
        self.substitute(Index::top(), term)
    }

    pub(crate) fn normalize<U: Definitions>(
        &mut self,
        definitions: &U,
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
                        body.substitute_top(expression);
                        body.normalize(definitions)?;
                        *self = *body.clone();
                    }
                    Duplicate {
                        binding: new_binding,
                        expression,
                        body: new_body,
                    } => {
                        body.shift(Index::top().child());
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
                        argument.shift_top();
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
                        body.substitute_top(argument);
                        body.normalize(definitions)?;
                        *self = *body;
                    }
                    _ => {
                        argument.normalize(definitions)?;
                    }
                }
            }
            Variable(_) => {}
            _ => todo!("handle typed terms"),
        }

        Ok(())
    }
}
