use std::{fmt::Debug, mem::replace};

use super::{alloc::Reallocate, Allocator, Definitions, Index, IntoInner, Primitives, Term, Zero};

#[derive(Debug)]
pub enum NormalizationError {
    InvalidDuplication,
    InvalidApplication,
}

impl<T, V: Primitives<T>, A: Allocator<T, V>> Term<T, V, A> {
    pub(crate) fn shift(&mut self, replaced: Index) {
        use Term::*;

        match self {
            Variable(index) => {
                if !index.is_below(replaced) {
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
            Reference(_) | Primitive(_) | Universe => {}

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

    pub fn substitute_in(&mut self, variable: Index, term: &Term<T, V, A>, alloc: &A)
    where
        T: Clone,
        V: Clone,
    {
        use Term::*;

        match self {
            Variable(idx) => {
                if variable == *idx {
                    *self = alloc.copy(term);
                } else if idx.is_above(variable) {
                    *idx = idx.parent();
                }
            }
            Lambda { body, .. } => {
                body.substitute_shifted_in(variable, term, alloc);
            }
            Apply {
                function, argument, ..
            } => {
                function.substitute_in(variable, term, alloc);
                argument.substitute_in(variable, term, alloc);
            }
            Put(expr) => {
                expr.substitute_in(variable, term, alloc);
            }
            Duplicate {
                body, expression, ..
            } => {
                expression.substitute_in(variable, term, alloc);
                body.substitute_shifted_in(variable, term, alloc);
            }
            Reference(_) | Universe => {}
            Primitive(_) => todo!(),

            Wrap(expr) => expr.substitute_in(variable, term, alloc),
            Annotation { expression, ty, .. } => {
                expression.substitute_in(variable, term, alloc);
                ty.substitute_in(variable, term, alloc);
            }
            Function {
                argument_type,
                return_type,
                ..
            } => {
                argument_type.substitute_in(variable, term, alloc);

                let mut term = alloc.copy(term);
                term.shift_top();
                term.shift_top();
                return_type.substitute_in(variable.child().child(), &term, alloc);
            }
        }
    }

    pub(crate) fn substitute_shifted_in(&mut self, variable: Index, term: &Term<T, V, A>, alloc: &A)
    where
        T: Clone,
        V: Clone,
    {
        let mut term = alloc.copy(term);
        term.shift_top();
        self.substitute_in(variable.child(), &term, alloc)
    }

    pub fn substitute_top_in(&mut self, term: &Term<T, V, A>, alloc: &A)
    where
        T: Clone,
        V: Clone,
    {
        self.substitute_in(Index::top(), term, alloc)
    }

    pub(crate) fn weak_normalize_in_erased<U: Definitions<T, V, B>, B: Allocator<T, V>>(
        &mut self,
        definitions: &U,
        alloc: &A,
        erase: bool,
    ) -> Result<(), NormalizationError>
    where
        T: Clone,
        V: Clone,
        A: Reallocate<T, V, B>,
    {
        use Term::*;

        match self {
            Reference(binding) => {
                if let Some(term) = definitions.get(binding).map(|term| {
                    let mut term = alloc.reallocating_copy(term.as_ref());
                    term.weak_normalize_in_erased(definitions, alloc, erase)?;
                    Ok(term)
                }) {
                    *self = term?;
                }
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                function.weak_normalize_in_erased(definitions, alloc, erase)?;
                let f = alloc.copy(&*function);
                match f {
                    Put(_) => Err(NormalizationError::InvalidApplication)?,
                    Duplicate { body, expression } => {
                        let mut argument = alloc.copy_boxed(argument);
                        argument.shift_top();
                        let body = alloc.alloc(Apply {
                            function: body,
                            argument,
                            erased: *erased,
                        });
                        *self = Duplicate { expression, body };
                    }
                    Lambda { mut body, .. } => {
                        body.substitute_top_in(argument, alloc);
                        body.weak_normalize_in_erased(definitions, alloc, erase)?;
                        *self = body.into_inner();
                    }
                    Primitive(prim) => {
                        *self = prim.apply(argument, alloc);
                    }
                    _ => {}
                }
            }

            Put(term) if erase => {
                term.weak_normalize_in_erased(definitions, alloc, erase)?;
                *self = replace(term, Term::Universe);
            }

            Duplicate { body, expression } if erase => {
                body.substitute_top_in(expression, alloc);
                body.weak_normalize_in_erased(definitions, alloc, erase)?;
                *self = replace(body, Term::Universe);
            }

            Variable(_) | Primitive(_) | Lambda { .. } | Put(_) => {}

            Duplicate { body, expression } => {
                expression.weak_normalize_in_erased(definitions, alloc, erase)?;

                match &mut **expression {
                    Put(term) => {
                        body.substitute_top_in(term, alloc);
                        body.weak_normalize_in_erased(definitions, alloc, erase)?;
                        *self = replace(body, Term::Universe);
                    }
                    Duplicate {
                        body: sub_body,
                        expression: sub_expression,
                    } => {
                        body.shift(Index::top().child());
                        let dup = Duplicate {
                            body: alloc.copy_boxed(body),
                            expression: alloc.copy_boxed(sub_body),
                        };
                        *self = Duplicate {
                            expression: alloc.alloc(replace(sub_expression, Term::Universe)),
                            body: alloc.alloc(dup),
                        };
                    }
                    _ => {}
                }
            }

            Universe | Function { .. } | Wrap(_) => {}
            Annotation { expression, .. } => {
                expression.weak_normalize_in_erased(definitions, alloc, erase)?;
                *self = replace(expression, Term::Universe);
            }
        }

        Ok(())
    }

    pub fn weak_normalize_in<U: Definitions<T, V, B>, B: Allocator<T, V>>(
        &mut self,
        definitions: &U,
        alloc: &A,
    ) -> Result<(), NormalizationError>
    where
        T: Clone,
        V: Clone,
        A: Reallocate<T, V, B>,
    {
        self.weak_normalize_in_erased(definitions, alloc, false)
    }

    pub fn normalize_in<U: Definitions<T, V, B>, B: Allocator<T, V>>(
        &mut self,
        definitions: &U,
        alloc: &A,
    ) -> Result<(), NormalizationError>
    where
        T: Clone,
        V: Clone,
        A: Reallocate<T, V, B>,
    {
        use Term::*;

        match self {
            Reference(binding) => {
                if let Some(term) = definitions.get(binding).map(|term| {
                    let mut term = alloc.reallocating_copy(term.as_ref());
                    term.normalize_in(definitions, alloc)?;
                    Ok(term)
                }) {
                    *self = term?;
                }
            }
            Lambda { body, erased, .. } => {
                body.normalize_in(definitions, alloc)?;
                if *erased {
                    body.substitute_top_in(&Term::Variable(Index::top()), alloc);
                    *self = replace(&mut *body, Universe);
                }
            }
            Put(term) => {
                term.normalize_in(definitions, alloc)?;
                *self = replace(term, Term::Universe);
            }
            Duplicate { body, expression } => {
                body.substitute_top_in(expression, alloc);
                body.normalize_in(definitions, alloc)?;
                *self = replace(body, Term::Universe);
            }
            Apply {
                function,
                argument,
                erased,
            } => {
                function.normalize_in(definitions, alloc)?;
                let function = alloc.copy(function);
                if *erased {
                    *self = function;
                } else {
                    match function {
                        Put(_) => Err(NormalizationError::InvalidApplication)?,
                        Primitive(primitive) => {
                            *self = primitive.apply(argument, alloc);
                        }
                        Lambda { mut body, .. } => {
                            body.substitute_top_in(argument, alloc);
                            body.normalize_in(definitions, alloc)?;

                            *self = body.into_inner();
                        }
                        _ => {
                            argument.normalize_in(definitions, alloc)?;
                        }
                    }
                }
            }
            Variable(_) | Universe | Primitive(_) | Wrap(_) | Function { .. } => {}

            Annotation { expression, .. } => {
                expression.normalize_in(definitions, alloc)?;
                *self = replace(expression, Term::Universe);
            }
        }

        Ok(())
    }

    pub fn normalize<U: Definitions<T, V, B>, B: Allocator<T, V>>(
        &mut self,
        definitions: &U,
    ) -> Result<(), NormalizationError>
    where
        T: Clone,
        V: Clone,
        A: Zero + Reallocate<T, V, B>,
    {
        let alloc = A::zero();
        self.normalize_in(definitions, &alloc)
    }

    pub fn weak_normalize<U: Definitions<T, V, A>>(
        &mut self,
        definitions: &U,
    ) -> Result<(), NormalizationError>
    where
        T: Clone + Debug,
        V: Clone,
        A: Zero + Reallocate<T, V, A>,
    {
        let alloc = A::zero();
        self.weak_normalize_in(definitions, &alloc)
    }

    pub fn substitute_top(&mut self, term: &Term<T, V, A>)
    where
        T: Clone,
        V: Clone,
        A: Zero,
    {
        let alloc = A::zero();

        self.substitute_top_in(term, &alloc)
    }

    pub fn substitute(&mut self, variable: Index, term: &Term<T, V, A>)
    where
        T: Clone,
        V: Clone,
        A: Zero,
    {
        let alloc = A::zero();

        self.substitute_in(variable, term, &alloc)
    }
}
