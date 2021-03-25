use super::{Index, Term};

impl Eq for Term {}

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        use Term::*;

        fn eq_helper(a: &Term, b: &Term, index: Index) -> bool {
            match (a, b) {
                (Variable(a), Variable(b)) => a == b,
                (Lambda { body: a_body, .. }, Lambda { body: b_body, .. }) => {
                    let mut a_body = a_body.clone();
                    a_body.substitute_top(&Term::Variable(index));
                    let mut b_body = b_body.clone();
                    b_body.substitute_top(&Term::Variable(index));
                    eq_helper(&a_body, &b_body, index.child())
                }
                (
                    Apply {
                        function: a_function,
                        argument: a_argument,
                    },
                    Apply {
                        function: b_function,
                        argument: b_argument,
                    },
                ) => {
                    eq_helper(a_function, b_function, index)
                        && eq_helper(a_argument, b_argument, index)
                }
                (Put(a), Put(b)) => eq_helper(a, b, index),
                (
                    Duplicate {
                        expression: _,
                        body: _,
                        ..
                    },
                    Duplicate {
                        expression: _,
                        body: _,
                        ..
                    },
                ) => {
                    todo!("equality for duplicate")
                }
                (Reference(a), Reference(b)) => a == b,

                (
                    Function {
                        return_type: a_return_type,
                        argument_type: a_argument_type,
                        self_binding: a_self_binding,
                        ..
                    },
                    Function {
                        return_type: b_return_type,
                        argument_type: b_argument_type,
                        self_binding: b_self_binding,
                        ..
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
                        && a_self_binding == b_self_binding
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
                (Wrap(a), Wrap(b)) => eq_helper(a, b, index),

                _ => false,
            }
        }

        eq_helper(self, other, Index::top())
    }
}
