use super::{Index, Term};

impl<T: Eq + Clone> Eq for Term<T> {}

impl<T: PartialEq + Clone> PartialEq for Term<T> {
    fn eq(&self, other: &Self) -> bool {
        use Term::*;

        fn eq_helper<T: PartialEq + Clone>(a: &Term<T>, b: &Term<T>, index: Index) -> bool {
            match (a, b) {
                (Variable(a), Variable(b)) => a == b,
                (
                    Lambda {
                        body: a_body,
                        erased: a_erased,
                    },
                    Lambda {
                        body: b_body,
                        erased: b_erased,
                    },
                ) => {
                    let mut a_body = a_body.clone();
                    a_body.substitute_top(&Term::Variable(index));
                    let mut b_body = b_body.clone();
                    b_body.substitute_top(&Term::Variable(index));
                    eq_helper(&a_body, &b_body, index.child()) && a_erased == b_erased
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
                    eq_helper(a_function, b_function, index)
                        && eq_helper(a_argument, b_argument, index)
                        && a_erased == b_erased
                }
                (Put(a), Put(b)) => eq_helper(a, b, index),
                (
                    Duplicate {
                        expression: a_expression,
                        body: a_body,
                    },
                    Duplicate {
                        expression: b_expression,
                        body: b_body,
                    },
                ) => {
                    let mut a_body = a_body.clone();
                    a_body.substitute_top(&Term::Variable(index));
                    let mut b_body = b_body.clone();
                    b_body.substitute_top(&Term::Variable(index));
                    eq_helper(&a_body, &b_body, index.child()) && a_expression == b_expression
                }
                (Reference(a), Reference(b)) => a == b,

                (
                    Function {
                        return_type: a_return_type,
                        argument_type: a_argument_type,
                        erased: a_erased,
                    },
                    Function {
                        return_type: b_return_type,
                        argument_type: b_argument_type,
                        erased: b_erased,
                    },
                ) => {
                    let mut a_return_type = a_return_type.clone();
                    a_return_type.substitute(Index::top().child(), &Term::Variable(index));
                    a_return_type.substitute_top(&Term::Variable(index.child()));
                    let mut b_return_type = b_return_type.clone();
                    b_return_type.substitute(Index::top().child(), &Term::Variable(index));
                    b_return_type.substitute_top(&Term::Variable(index.child()));
                    eq_helper(&a_return_type, &b_return_type, index.child().child())
                        && eq_helper(a_argument_type, b_argument_type, index)
                        && (a_erased == b_erased)
                }
                (Universe, Universe) => true,
                (
                    Annotation {
                        expression: expression_a,
                        ..
                    },
                    Annotation {
                        expression: expression_b,
                        ..
                    },
                ) => eq_helper(expression_a, expression_b, index),
                (
                    Annotation {
                        expression: expression_a,
                        ..
                    },
                    expression_b,
                ) => eq_helper(expression_a, expression_b, index),
                (
                    expression_a,
                    Annotation {
                        expression: expression_b,
                        ..
                    },
                ) => eq_helper(expression_a, expression_b, index),
                (Wrap(a), Wrap(b)) => eq_helper(a, b, index),

                _ => false,
            }
        }

        eq_helper(self, other, Index::top())
    }
}
