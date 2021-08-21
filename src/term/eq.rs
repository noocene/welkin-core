use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use derivative::Derivative;

use crate::analysis::Empty;

use super::{
    alloc::Reallocate, Allocator, Definitions, IntoInner, NormalizationError, Primitives, Show,
    Term, Zero,
};

use bumpalo::{boxed::Box as BumpBox, Bump};

#[derive(Derivative)]
#[derivative(Debug(bound = "T: Show, V: Show"))]
enum EqualityTree<'a, T, V: Primitives<T>, A: Allocator<T, V>> {
    Equal(Term<T, V, A>, Term<T, V, A>),
    Or(BumpBox<'a, Option<(EqualityTree<'a, T, V, A>, EqualityTree<'a, T, V, A>)>>),
    And(BumpBox<'a, Option<(EqualityTree<'a, T, V, A>, EqualityTree<'a, T, V, A>)>>),
    Leaf(bool),
}

pub trait EqualityCache {
    fn register(&mut self, a: u64, b: u64, checks: bool);
    fn check(&self, a: u64, b: u64) -> Option<bool>;
}

#[derive(Copy, Clone)]
pub struct NullCache;

pub struct MapCache {
    data: HashMap<(u64, u64), bool>,
}

impl MapCache {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl EqualityCache for MapCache {
    fn register(&mut self, a: u64, b: u64, checks: bool) {
        self.data.insert((a, b), checks);
    }

    fn check(&self, a: u64, b: u64) -> Option<bool> {
        self.data.get(&(a, b)).cloned()
    }
}

impl EqualityCache for NullCache {
    fn register(&mut self, _: u64, _: u64, _: bool) {}

    fn check(&self, _: u64, _: u64) -> Option<bool> {
        None
    }
}

impl<'a, T: EqualityCache> EqualityCache for &'a mut T {
    fn register(&mut self, a: u64, b: u64, checks: bool) {
        T::register(self, a, b, checks)
    }

    fn check(&self, a: u64, b: u64) -> Option<bool> {
        T::check(self, a, b)
    }
}

impl<T: PartialEq + Show + Clone, V: Show + Clone + Primitives<T>, A: Allocator<T, V>>
    Term<T, V, A>
{
    pub fn equals(&self, other: &Self) -> bool
    where
        V: PartialEq,
    {
        match (self, other) {
            (Term::Variable(a), Term::Variable(b)) => a == b,
            (
                Term::Lambda { body, erased },
                Term::Lambda {
                    body: b_body,
                    erased: b_erased,
                },
            ) => body.equals(b_body) && erased == b_erased,
            (
                Term::Apply {
                    function,
                    argument,
                    erased,
                },
                Term::Apply {
                    function: b_function,
                    argument: b_argument,
                    erased: b_erased,
                },
            ) => function.equals(b_function) && argument.equals(b_argument) && erased == b_erased,
            (Term::Put(a), Term::Put(b)) => a.equals(b),
            (
                Term::Duplicate { expression, body },
                Term::Duplicate {
                    expression: b_expression,
                    body: b_body,
                },
            ) => expression.equals(b_expression) && body.equals(b_body),
            (Term::Reference(a), Term::Reference(b)) => a == b,
            (Term::Primitive(a), Term::Primitive(b)) => a == b,
            (Term::Universe, Term::Universe) => true,
            (
                Term::Function {
                    argument_type,
                    return_type,
                    erased,
                },
                Term::Function {
                    argument_type: b_argument_type,
                    return_type: b_return_type,
                    erased: b_erased,
                },
            ) => {
                argument_type.equals(b_argument_type)
                    && return_type.equals(b_return_type)
                    && erased == b_erased
            }
            (
                Term::Annotation {
                    checked,
                    expression,
                    ty,
                },
                Term::Annotation {
                    checked: b_checked,
                    expression: b_expression,
                    ty: b_ty,
                },
            ) => expression.equals(b_expression) && ty.equals(b_ty) && checked == b_checked,
            (Term::Wrap(a), Term::Wrap(b)) => a.equals(b),
            _ => false,
        }
    }

    pub fn equivalent_in<U: Definitions<T, V, B>, B: Allocator<T, V>>(
        &self,
        other: &Self,
        definitions: &U,
        alloc: &A,
        cache: &mut impl EqualityCache,
    ) -> Result<bool, NormalizationError>
    where
        A: Reallocate<T, V, B>,
        T: Hash,
        V: Hash,
    {
        use Term::*;

        fn equivalence_helper<
            'b,
            U: Definitions<T, V, B>,
            T: Show + PartialEq + Clone + Hash,
            V: Show + Primitives<T> + Clone + Hash,
            A: Allocator<T, V> + Reallocate<T, V, B>,
            B: Allocator<T, V>,
        >(
            tree: EqualityTree<'b, T, V, A>,
            definitions: &U,
            alloc: &A,
            o_alloc: &'b Bump,
            cache: &mut impl EqualityCache,
        ) -> Result<EqualityTree<'b, T, V, A>, NormalizationError> {
            Ok(match tree {
                this @ EqualityTree::Leaf(_) => this,
                EqualityTree::And(mut data) => {
                    let (a, b) = data.as_ref().as_ref().unwrap();
                    match (a, b) {
                        (EqualityTree::Leaf(false), _) | (_, EqualityTree::Leaf(false)) => {
                            EqualityTree::Leaf(false)
                        }
                        (EqualityTree::Leaf(true), EqualityTree::Leaf(true)) => {
                            EqualityTree::Leaf(true)
                        }
                        _ => EqualityTree::And(BumpBox::new_in(
                            Some({
                                let data = data.take().unwrap();
                                (
                                    equivalence_helper(
                                        data.0,
                                        definitions,
                                        alloc,
                                        o_alloc,
                                        &mut *cache,
                                    )?,
                                    equivalence_helper(data.1, definitions, alloc, o_alloc, cache)?,
                                )
                            }),
                            o_alloc,
                        )),
                    }
                }
                EqualityTree::Or(mut data) => {
                    let (a, b) = data.as_ref().as_ref().unwrap();
                    match (&a, &b) {
                        (EqualityTree::Leaf(true), _) | (_, EqualityTree::Leaf(true)) => {
                            EqualityTree::Leaf(true)
                        }
                        (EqualityTree::Leaf(false), EqualityTree::Leaf(false)) => {
                            EqualityTree::Leaf(false)
                        }
                        _ => EqualityTree::Or(BumpBox::new_in(
                            {
                                let data = data.take().unwrap();
                                Some((
                                    equivalence_helper(
                                        data.0,
                                        definitions,
                                        alloc,
                                        o_alloc,
                                        &mut *cache,
                                    )?,
                                    equivalence_helper(data.1, definitions, alloc, o_alloc, cache)?,
                                ))
                            },
                            o_alloc,
                        )),
                    }
                }
                EqualityTree::Equal(mut a, mut b) => {
                    a.weak_normalize_in_erased::<_, B>(&Empty, alloc, true)?;
                    b.weak_normalize_in_erased::<_, B>(&Empty, alloc, true)?;

                    let mut hasher = DefaultHasher::new();

                    a.hash(&mut hasher);

                    let a_hash = hasher.finish();

                    let mut hasher = DefaultHasher::new();

                    b.hash(&mut hasher);

                    let b_hash = hasher.finish();

                    if a_hash == b_hash {
                        return Ok(EqualityTree::Leaf(true));
                    }

                    if let Some(leaf) = cache.check(a_hash, b_hash) {
                        return Ok(EqualityTree::Leaf(leaf));
                    }

                    let mut ret_a = None;

                    match (&a, &b) {
                        (
                            Apply {
                                function: a_function,
                                argument: a_argument,
                                erased: a_erased,
                            },
                            Apply {
                                function: b_function,
                                argument: b_argument,
                                erased: b_erased,
                            },
                        ) => {
                            ret_a = if a_erased != b_erased {
                                Some(EqualityTree::Leaf(false))
                            } else {
                                Some(EqualityTree::And(BumpBox::new_in(
                                    Some((
                                        EqualityTree::Equal(
                                            alloc.copy(a_function),
                                            alloc.copy(b_function),
                                        ),
                                        EqualityTree::Equal(
                                            alloc.copy(a_argument),
                                            alloc.copy(b_argument),
                                        ),
                                    )),
                                    o_alloc,
                                )))
                            }
                        }
                        (Reference(a), Reference(b)) => {
                            if a == b {
                                ret_a = Some(EqualityTree::Leaf(true))
                            }
                        }

                        _ => {}
                    }

                    a.weak_normalize_in_erased::<_, B>(definitions, alloc, true)?;
                    b.weak_normalize_in_erased::<_, B>(definitions, alloc, true)?;

                    let ret_b = match (a, b) {
                        (Universe, Universe) => EqualityTree::Leaf(true),
                        (
                            Function {
                                argument_type: a_argument_type,
                                return_type: a_return_type,
                                erased: a_erased,
                            },
                            Function {
                                argument_type: b_argument_type,
                                return_type: b_return_type,
                                erased: b_erased,
                            },
                        ) => {
                            if a_erased != b_erased {
                                EqualityTree::Leaf(false)
                            } else {
                                EqualityTree::And(BumpBox::new_in(
                                    Some((
                                        EqualityTree::Equal(
                                            a_argument_type.into_inner(),
                                            b_argument_type.into_inner(),
                                        ),
                                        EqualityTree::Equal(
                                            a_return_type.into_inner(),
                                            b_return_type.into_inner(),
                                        ),
                                    )),
                                    o_alloc,
                                ))
                            }
                        }
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
                            if a_erased != b_erased {
                                EqualityTree::Leaf(false)
                            } else {
                                EqualityTree::Equal(a_body.into_inner(), b_body.into_inner())
                            }
                        }
                        (
                            Apply {
                                argument: a_argument,
                                function: a_function,
                                erased: a_erased,
                            },
                            Apply {
                                argument: b_argument,
                                function: b_function,
                                erased: b_erased,
                            },
                        ) => {
                            if a_erased != b_erased {
                                EqualityTree::Leaf(false)
                            } else {
                                EqualityTree::And(BumpBox::new_in(
                                    Some((
                                        EqualityTree::Equal(
                                            a_argument.into_inner(),
                                            b_argument.into_inner(),
                                        ),
                                        EqualityTree::Equal(
                                            a_function.into_inner(),
                                            b_function.into_inner(),
                                        ),
                                    )),
                                    o_alloc,
                                ))
                            }
                        }
                        (Variable(a), Variable(b)) => EqualityTree::Leaf(a == b),
                        (Wrap(a), Wrap(b)) => EqualityTree::Equal(a.into_inner(), b.into_inner()),
                        (Put(a), Put(b)) => EqualityTree::Equal(a.into_inner(), b.into_inner()),
                        (
                            Duplicate {
                                expression: a_expression,
                                body: a_body,
                            },
                            Duplicate {
                                expression: b_expression,
                                body: b_body,
                            },
                        ) => EqualityTree::And(BumpBox::new_in(
                            Some((
                                EqualityTree::Equal(
                                    a_expression.into_inner(),
                                    b_expression.into_inner(),
                                ),
                                EqualityTree::Equal(a_body.into_inner(), b_body.into_inner()),
                            )),
                            o_alloc,
                        )),
                        _ => EqualityTree::Leaf(false),
                    };

                    if let Some(ret_a) = ret_a {
                        EqualityTree::Or(BumpBox::new_in(Some((ret_a, ret_b)), o_alloc))
                    } else {
                        ret_b
                    }
                }
            })
        }

        let o_alloc = Bump::new();

        let mut a = alloc.copy(self);
        let mut b = alloc.copy(other);

        a.weak_normalize_in_erased(definitions, alloc, true)?;
        b.weak_normalize_in_erased(definitions, alloc, true)?;

        let mut hasher = DefaultHasher::new();

        a.hash(&mut hasher);

        let a_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();

        b.hash(&mut hasher);

        let b_hash = hasher.finish();

        if a_hash == b_hash {
            return Ok(true);
        }

        if let Some(leaf) = cache.check(a_hash, b_hash) {
            return Ok(leaf);
        }

        let mut equality = EqualityTree::Equal(a, b);

        while match equality {
            EqualityTree::Leaf(_) => false,
            _ => true,
        } {
            equality = equivalence_helper(equality, definitions, alloc, &o_alloc, cache)?;
        }

        Ok(if let EqualityTree::Leaf(leaf) = equality {
            cache.register(a_hash, b_hash, leaf);
            leaf
        } else {
            panic!()
        })
    }

    pub fn equivalent<U: Definitions<T, V, A>>(
        &self,
        other: &Self,
        definitions: &U,
        cache: &mut impl EqualityCache,
    ) -> Result<bool, NormalizationError>
    where
        A: Zero + Reallocate<T, V, A>,
        T: Hash,
        V: Hash,
    {
        let alloc = A::zero();

        self.equivalent_in(other, definitions, &alloc, cache)
    }
}
