use std::{fmt::Debug, mem::replace};

use super::{alloc::Reallocate, Allocator, Definitions, Index, IntoInner, Primitives, Term, Zero};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum NormalizationError {
    InvalidDuplication,
    InvalidApplication,
}

impl<T, V: Primitives<T>, A: Allocator<T, V>> Term<T, V, A> {
    pub(crate) fn shift(&mut self, replaced: Index) {
        self.shift_by(replaced, 1);
    }

    pub(crate) fn shift_by(&mut self, replaced: Index, by: isize) {
        use Term::*;

        match self {
            Variable(index) => {
                if !index.is_below(replaced) {
                    if by > 0 {
                        index.0 += by as usize;
                    } else {
                        index.0 -= by.abs() as usize;
                    }
                }
            }
            Lambda { body, .. } => body.shift_by(replaced.child(), by),
            Apply {
                function, argument, ..
            } => {
                function.shift_by(replaced, by);
                argument.shift_by(replaced, by);
            }
            Put(term) => {
                term.shift_by(replaced, by);
            }
            Duplicate {
                expression, body, ..
            } => {
                expression.shift_by(replaced, by);
                body.shift_by(replaced.child(), by);
            }
            Reference(_) | Primitive(_) | Universe => {}

            Wrap(term) => term.shift_by(replaced, by),
            Annotation { expression, ty, .. } => {
                expression.shift_by(replaced, by);
                ty.shift_by(replaced, by);
            }
            Function {
                argument_type,
                return_type,
                ..
            } => {
                argument_type.shift_by(replaced, by);
                return_type.shift_by(replaced.child().child(), by);
            }
        }
    }

    pub(crate) fn shift_top_by(&mut self, by: isize) {
        self.shift_by(Index::top(), by)
    }

    pub(crate) fn shift_top(&mut self) {
        self.shift_top_by(1);
    }

    pub(crate) fn substitute_in(
        &mut self,
        variable: Index,
        term: &Term<T, V, A>,
        alloc: &A,
        shift: bool,
    ) where
        T: Clone,
        V: Clone,
    {
        use Term::*;

        match self {
            Variable(idx) => {
                if variable == *idx {
                    *self = alloc.copy(term);
                } else if idx.is_above(variable) {
                    if shift {
                        *idx = idx.parent();
                    }
                }
            }
            Lambda { body, .. } => {
                let mut term = alloc.copy(term);
                term.shift_top();
                body.substitute_in(variable.child(), &term, alloc, shift);
            }
            Apply {
                function, argument, ..
            } => {
                function.substitute_in(variable, term, alloc, shift);
                argument.substitute_in(variable, term, alloc, shift);
            }
            Put(expr) => {
                expr.substitute_in(variable, term, alloc, shift);
            }
            Duplicate {
                body, expression, ..
            } => {
                expression.substitute_in(variable, term, alloc, shift);
                let mut term = alloc.copy(term);
                term.shift_top();
                body.substitute_in(variable.child(), &term, alloc, shift);
            }
            Reference(_) | Universe => {}
            Primitive(_) => todo!(),

            Wrap(expr) => expr.substitute_in(variable, term, alloc, shift),
            Annotation { expression, ty, .. } => {
                expression.substitute_in(variable, term, alloc, shift);
                ty.substitute_in(variable, term, alloc, shift);
            }
            Function {
                argument_type,
                return_type,
                ..
            } => {
                argument_type.substitute_in(variable, term, alloc, shift);

                let mut term = alloc.copy(term);
                term.shift_top_by(2);
                return_type.substitute_in(variable.child().child(), &term, alloc, shift);
            }
        }
    }

    pub fn substitute_top_in(&mut self, term: &Term<T, V, A>, alloc: &A)
    where
        T: Clone,
        V: Clone,
    {
        self.substitute_in(Index::top(), term, alloc, true)
    }

    pub(crate) fn substitute_top_in_unshifted(&mut self, term: &Term<T, V, A>, alloc: &A)
    where
        T: Clone,
        V: Clone,
    {
        self.substitute_in(Index::top(), term, alloc, false)
    }

    pub(crate) fn substitute_function_in(
        &mut self,
        mut self_binding: Term<T, V, A>,
        argument_binding: &Term<T, V, A>,
        alloc: &A,
    ) where
        T: Clone,
        V: Clone,
    {
        self_binding.shift_top();
        self.substitute_in(Index::top().child(), &self_binding, alloc, true);
        self.substitute_in(Index::top(), argument_binding, alloc, true);
    }

    pub(crate) fn substitute_function_in_unshifted(
        &mut self,
        mut self_binding: Term<T, V, A>,
        argument_binding: &Term<T, V, A>,
        alloc: &A,
    ) where
        T: Clone,
        V: Clone,
    {
        self_binding.shift_top();
        self.substitute_in(Index::top().child(), &self_binding, alloc, true);
        self.substitute_in(Index::top(), argument_binding, alloc, false);
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

    pub(crate) fn extract_from_annotation(self) -> Self {
        if let Term::Annotation { expression, .. } = self {
            expression.into_inner().extract_from_annotation()
        } else {
            self
        }
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

        self.substitute_in(variable, term, &alloc, true)
    }
}
