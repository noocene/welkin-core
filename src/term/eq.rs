use crate::analysis::Empty;

use super::{
    alloc::Reallocate, Allocator, Definitions, IntoInner, NormalizationError, Primitives, Show,
    Term, Zero,
};

impl<T: PartialEq + Show + Clone, V: Show + Clone + Primitives<T>, A: Allocator<T, V>>
    Term<T, V, A>
{
    pub fn equivalent_in<U: Definitions<T, V, B>, B: Allocator<T, V>>(
        &self,
        other: &Self,
        definitions: &U,
        alloc: &A,
    ) -> Result<bool, NormalizationError>
    where
        A: Reallocate<T, V, B>,
    {
        use Term::*;

        fn equivalence_helper<
            U: Definitions<T, V, B>,
            T: Show + PartialEq + Clone,
            V: Show + Primitives<T> + Clone,
            A: Allocator<T, V> + Reallocate<T, V, B>,
            B: Allocator<T, V>,
        >(
            mut a: A::Box,
            mut b: A::Box,
            definitions: &U,
            alloc: &A,
        ) -> Result<bool, NormalizationError> {
            let mut eq = {
                a.lazy_normalize_in::<_, B>(&Empty, alloc)?;
                b.lazy_normalize_in::<_, B>(&Empty, alloc)?;

                match (&*a, &*b) {
                    (
                        Apply {
                            function: a_function,
                            argument: a_argument,
                            ..
                        },
                        Apply {
                            function: b_function,
                            argument: b_argument,
                            ..
                        },
                    ) => {
                        equivalence_helper(
                            alloc.copy_boxed(a_function),
                            alloc.copy_boxed(b_function),
                            definitions,
                            alloc,
                        )? && equivalence_helper(
                            alloc.copy_boxed(a_argument),
                            alloc.copy_boxed(b_argument),
                            definitions,
                            alloc,
                        )?
                    }
                    (Reference(a), Reference(b)) => a == b,

                    _ => false,
                }
            };

            if !eq {
                a.lazy_normalize_in(definitions, alloc)?;
                b.lazy_normalize_in(definitions, alloc)?;

                eq = match (a.into_inner(), b.into_inner()) {
                    (Variable(a), Variable(b)) => a == b,
                    (Lambda { body: a_body, .. }, Lambda { body: b_body, .. }) => {
                        equivalence_helper(a_body, b_body, definitions, alloc)?
                    }
                    (Put(a), Put(b)) => equivalence_helper(a, b, definitions, alloc)?,
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
                        equivalence_helper(a_body, b_body, definitions, alloc)?
                            && equivalence_helper(a_expression, b_expression, definitions, alloc)?
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
                        a_erased == b_erased
                            && equivalence_helper(a_function, b_function, definitions, alloc)?
                            && equivalence_helper(a_argument, b_argument, definitions, alloc)?
                    }
                    (Reference(a), Reference(b)) => a == b,
                    (
                        Function {
                            return_type: a_return_type,
                            argument_type: a_argument_type,
                            ..
                        },
                        Function {
                            return_type: b_return_type,
                            argument_type: b_argument_type,
                            ..
                        },
                    ) => {
                        equivalence_helper(a_return_type, b_return_type, definitions, alloc)?
                            && equivalence_helper(
                                a_argument_type,
                                b_argument_type,
                                definitions,
                                alloc,
                            )?
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
                    ) => equivalence_helper(a_expression, b_expression, definitions, alloc)?,
                    (
                        Annotation {
                            expression: a_expression,
                            ..
                        },
                        b_expression,
                    ) => equivalence_helper(
                        a_expression,
                        alloc.alloc(b_expression),
                        definitions,
                        alloc,
                    )?,
                    (
                        a_expression,
                        Annotation {
                            expression: b_expression,
                            ..
                        },
                    ) => equivalence_helper(
                        alloc.alloc(a_expression),
                        b_expression,
                        definitions,
                        alloc,
                    )?,
                    (Wrap(a), Wrap(b)) => equivalence_helper(a, b, definitions, alloc)?,

                    (a, b) => {
                        println!("{:?} != {:?}", a, b);
                        false
                    }
                };
            }

            Ok(eq)
        }

        let a = alloc.alloc(alloc.copy(self));
        let b = alloc.alloc(alloc.copy(other));

        equivalence_helper(a, b, definitions, alloc)
    }

    pub fn equivalent<U: Definitions<T, V, A>>(
        &self,
        other: &Self,
        definitions: &U,
    ) -> Result<bool, NormalizationError>
    where
        A: Zero + Reallocate<T, V, A>,
    {
        let alloc = A::zero();

        self.equivalent_in(other, definitions, &alloc)
    }
}
