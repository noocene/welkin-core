use super::{Definitions, Index, NormalizationError, Primitives, Show, Term};

impl<T: PartialEq + Show + Clone, V: Show + Clone + Primitives<T>> Term<T, V> {
    pub fn equivalent<U: Definitions<T, V>>(
        &self,
        other: &Self,
        definitions: &U,
    ) -> Result<bool, NormalizationError> {
        use Term::*;

        fn equivalence_helper<
            U: Definitions<T, V>,
            T: Show + PartialEq + Clone,
            V: Show + Primitives<T> + Clone,
        >(
            mut a: Box<Term<T, V>>,
            mut b: Box<Term<T, V>>,
            index: Index,
            definitions: &U,
        ) -> Result<bool, NormalizationError> {
            match (&*a, &*b) {
                (Apply { .. }, Apply { .. }) => {}
                (_, Apply { .. }) => {
                    b.lazy_normalize(definitions)?;
                }
                (Apply { .. }, _) => {
                    a.lazy_normalize(definitions)?;
                }
                _ => {}
            }

            match (&*a, &*b) {
                (Reference(_), Reference(_)) => {}
                (Reference(_), _) => {
                    a.lazy_normalize(definitions)?;
                }
                (_, Reference(_)) => {
                    b.lazy_normalize(definitions)?;
                }
                _ => {}
            }

            Ok(match (*a, *b) {
                (Variable(a), Variable(b)) => a == b,
                (
                    Lambda {
                        body: mut a_body,
                        erased: a_erased,
                    },
                    Lambda {
                        body: mut b_body,
                        erased: b_erased,
                    },
                ) => {
                    a_body.substitute_top(&Term::Variable(index));
                    b_body.substitute_top(&Term::Variable(index));
                    equivalence_helper(a_body, b_body, index.child(), definitions)?
                        && a_erased == b_erased
                }
                (Put(a), Put(b)) => equivalence_helper(a, b, index, definitions)?,
                (
                    Duplicate {
                        expression: a_expression,
                        body: mut a_body,
                    },
                    Duplicate {
                        expression: b_expression,
                        body: mut b_body,
                    },
                ) => {
                    a_body.substitute_top(&Term::Variable(index));
                    b_body.substitute_top(&Term::Variable(index));
                    equivalence_helper(a_body, b_body, index.child(), definitions)?
                        && equivalence_helper(a_expression, b_expression, index, definitions)?
                }
                (
                    Apply {
                        function: a_function,
                        argument: a_argument,
                        erased: a_erased,
                    },
                    Apply {
                        function: b_function,
                        erased: b_erased,
                        argument: b_argument,
                    },
                ) => {
                    equivalence_helper(a_function, b_function, index, definitions)?
                        && a_erased == b_erased
                        && equivalence_helper(a_argument, b_argument, index, definitions)?
                }
                (Reference(a), Reference(b)) => a == b,
                (
                    Function {
                        return_type: mut a_return_type,
                        argument_type: a_argument_type,
                        erased: a_erased,
                    },
                    Function {
                        return_type: mut b_return_type,
                        argument_type: b_argument_type,
                        erased: b_erased,
                    },
                ) => {
                    a_return_type.substitute(Index::top().child(), &Term::Variable(index));
                    a_return_type.substitute_top(&Term::Variable(index.child()));
                    b_return_type.substitute(Index::top().child(), &Term::Variable(index));
                    b_return_type.substitute_top(&Term::Variable(index.child()));
                    equivalence_helper(
                        a_return_type,
                        b_return_type,
                        index.child().child(),
                        definitions,
                    )? && a_erased == b_erased
                        && equivalence_helper(a_argument_type, b_argument_type, index, definitions)?
                }
                (Universe, Universe) => true,
                (
                    Annotation {
                        expression: a_expression,
                        ..
                    },
                    Annotation {
                        expression: b_expression,
                        ..
                    },
                ) => equivalence_helper(a_expression, b_expression, index, definitions)?,
                (
                    Annotation {
                        expression: a_expression,
                        ..
                    },
                    b_expression,
                ) => equivalence_helper(a_expression, Box::new(b_expression), index, definitions)?,
                (
                    a_expression,
                    Annotation {
                        expression: b_expression,
                        ..
                    },
                ) => equivalence_helper(Box::new(a_expression), b_expression, index, definitions)?,
                (Wrap(a), Wrap(b)) => equivalence_helper(a, b, index, definitions)?,

                _ => false,
            })
        }

        let mut a = Box::new(self.clone());
        let mut b = Box::new(other.clone());

        a.lazy_normalize(definitions)?;
        b.lazy_normalize(definitions)?;

        equivalence_helper(a, b, Index::top(), definitions)
    }
}
