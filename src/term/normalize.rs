use std::mem::replace;

use super::{Definitions, Index, Term};

#[derive(Debug)]
pub enum NormalizationError {
    InvalidDuplication,
    InvalidApplication,
}

impl<T> Term<T> {
    pub(crate) fn shift(&mut self, replaced: Index) {
        use Term::*;

        match self {
            Variable(index) => {
                if index.within(replaced) || *index == replaced {
                    *index = index.child();
                }
            }
            Lambda { body, .. } => body.shift(replaced.child()),
            Apply {
                function, argument, ..
            } => {
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
            Reference(_) | Universe => {}
            Primitive(_) => todo!(),

            Wrap(term) => term.shift(replaced),
            Annotation { expression, ty, .. } => {
                expression.shift(replaced);
                ty.shift(replaced);
            }
            Function {
                argument_type,
                return_type,
                ..
            } => {
                argument_type.shift(replaced);
                return_type.shift(replaced.child().child());
            }
        }
    }

    pub(crate) fn shift_top(&mut self) {
        self.shift(Index::top())
    }

    pub(crate) fn substitute_shifted(&mut self, variable: Index, term: &Term<T>)
    where
        T: Clone,
    {
        let mut term = term.clone();
        term.shift_top();
        self.substitute(variable.child(), &term)
    }

    pub(crate) fn substitute(&mut self, variable: Index, term: &Term<T>)
    where
        T: Clone,
    {
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
            Apply {
                function, argument, ..
            } => {
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
            Reference(_) | Universe => {}
            Primitive(_) => todo!(),

            Wrap(expr) => expr.substitute(variable, term),
            Annotation { expression, ty, .. } => {
                expression.substitute(variable, term);
                ty.substitute(variable, term);
            }
            Function {
                argument_type,
                return_type,
                ..
            } => {
                argument_type.substitute(variable, term);

                let mut term = term.clone();
                term.shift_top();
                term.shift_top();
                return_type.substitute(variable.child().child(), &term);
            }
        }
    }

    pub(crate) fn substitute_top(&mut self, term: &Term<T>)
    where
        T: Clone,
    {
        self.substitute(Index::top(), term)
    }

    pub(crate) fn normalize<U: Definitions<T>>(
        &mut self,
        definitions: &U,
    ) -> Result<(), NormalizationError>
    where
        T: Clone,
    {
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
            Lambda { body, erased, .. } => {
                body.normalize(definitions)?;
                if *erased {
                    body.substitute_top(&Term::Variable(Index::top()));
                    *self = replace(&mut *body, Universe);
                }
            }
            Put(term) => {
                term.normalize(definitions)?;
            }
            Duplicate { body, expression } => {
                expression.normalize(definitions)?;
                match &**expression {
                    Put(expression) => {
                        body.substitute_top(expression);
                        body.normalize(definitions)?;
                        *self = replace(body, Universe);
                    }
                    Duplicate {
                        expression,
                        body: new_body,
                    } => {
                        body.shift(Index::top().child());
                        let dup = Duplicate {
                            body: body.clone(),
                            expression: new_body.clone(),
                        };
                        let mut term = Duplicate {
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
            Apply {
                function,
                argument,
                erased,
            } => {
                function.normalize(definitions)?;
                let function = function.clone();
                if *erased {
                    *self = *function;
                } else {
                    match *function {
                        Put(_) => Err(NormalizationError::InvalidApplication)?,
                        Duplicate { body, expression } => {
                            let mut argument = argument.clone();
                            argument.shift_top();
                            let body = Box::new(Apply {
                                function: body,
                                argument,
                                erased: *erased,
                            });
                            let mut term = Duplicate { expression, body };
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
            }
            Variable(_) => {}
            Primitive(_) => todo!(),

            Universe => {}
            Wrap(term) => {
                term.normalize(definitions)?;
            }
            Annotation { expression, .. } => {
                expression.normalize(definitions)?;
                *self = replace(expression, Term::Universe);
            }
            Function {
                argument_type,
                return_type,
                erased,
                ..
            } => {
                if !*erased {
                    argument_type.normalize(definitions)?;
                    return_type.normalize(definitions)?;
                }
            }
        }

        Ok(())
    }

    pub(crate) fn lazy_normalize<U: Definitions<T>>(
        &mut self,
        definitions: &U,
    ) -> Result<(), NormalizationError>
    where
        T: Clone,
    {
        use Term::*;

        match self {
            Reference(binding) => {
                if let Some(term) = definitions.get(binding).map(|term| {
                    let mut term = term.clone();
                    term.lazy_normalize(definitions)?;
                    Ok(term)
                }) {
                    *self = term?;
                }
            }
            Put(term) => {
                term.lazy_normalize(definitions)?;
            }
            Duplicate { body, expression } => {
                expression.lazy_normalize(definitions)?;
                match &**expression {
                    Put(expression) => {
                        body.substitute_top(expression);
                        body.lazy_normalize(definitions)?;
                        *self = *body.clone();
                    }
                    Duplicate {
                        expression,
                        body: new_body,
                    } => {
                        body.shift(Index::top().child());
                        let dup = Duplicate {
                            body: body.clone(),
                            expression: new_body.clone(),
                        };
                        let mut term = Duplicate {
                            expression: expression.clone(),
                            body: Box::new(dup),
                        };
                        term.lazy_normalize(definitions)?;
                        *self = term;
                    }
                    Lambda { .. } => Err(NormalizationError::InvalidDuplication)?,
                    _ => {
                        body.lazy_normalize(definitions)?;
                    }
                }
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                function.lazy_normalize(definitions)?;
                let function = function.clone();
                match *function {
                    Put(_) => Err(NormalizationError::InvalidApplication)?,
                    Duplicate { body, expression } => {
                        let mut argument = argument.clone();
                        argument.shift_top();
                        let body = Box::new(Apply {
                            function: body,
                            argument,
                            erased: *erased,
                        });
                        let mut term = Duplicate { expression, body };
                        term.lazy_normalize(definitions)?;
                        *self = term;
                    }
                    Lambda { mut body, .. } => {
                        body.substitute_top(argument);
                        body.lazy_normalize(definitions)?;
                        *self = *body;
                    }
                    _ => {
                        argument.lazy_normalize(definitions)?;
                    }
                }
            }
            Variable(_) | Lambda { .. } => {}
            Primitive(_) => todo!(),

            Universe | Function { .. } => {}
            Wrap(term) => {
                term.lazy_normalize(definitions)?;
            }
            Annotation { expression, .. } => {
                expression.lazy_normalize(definitions)?;
                *self = replace(expression, Term::Universe);
            }
        }

        Ok(())
    }
}
