use std::ops::DerefMut;

use super::{Primitives, Term};

pub trait IntoInner<T> {
    fn into_inner(self) -> T;
}

pub trait Allocator<T, U: Primitives<T>>: Sized {
    type Box: DerefMut<Target = Term<T, U, Self>> + IntoInner<Term<T, U, Self>>;

    fn copy(&self, data: &Term<T, U, Self>) -> Term<T, U, Self>
    where
        T: Clone,
        U: Clone;

    fn copy_boxed(&self, data: &Self::Box) -> Self::Box
    where
        T: Clone,
        U: Clone;

    fn alloc(&self, data: Term<T, U, Self>) -> Self::Box;
}

pub trait Reallocate<T, U: Primitives<T>, A: Allocator<T, U>>: Allocator<T, U> {
    fn reallocate_boxed(&self, data: A::Box) -> Self::Box;
    fn reallocate(&self, data: Term<T, U, A>) -> Term<T, U, Self>;
    fn reallocating_copy(&self, data: &Term<T, U, A>) -> Term<T, U, Self>
    where
        T: Clone,
        U: Clone;
}

pub trait Zero {
    fn zero() -> Self;
}

#[derive(Clone, Copy, Debug)]
pub struct System;

impl Zero for System {
    fn zero() -> Self {
        System
    }
}

impl<T, U: Primitives<T>, A: Allocator<T, U>> IntoInner<Term<T, U, A>> for Box<Term<T, U, A>> {
    fn into_inner(self) -> Term<T, U, A> {
        *self
    }
}

impl<T, U: Primitives<T>> Reallocate<T, U, System> for System {
    fn reallocate_boxed(&self, data: Self::Box) -> Self::Box {
        data
    }

    fn reallocate(&self, data: Term<T, U, System>) -> Term<T, U, Self> {
        data
    }

    fn reallocating_copy(&self, data: &Term<T, U, System>) -> Term<T, U, Self>
    where
        T: Clone,
        U: Clone,
    {
        data.clone()
    }
}

impl<T, U: Primitives<T>> Allocator<T, U> for System {
    type Box = Box<Term<T, U>>;

    fn copy(&self, data: &Term<T, U>) -> Term<T, U>
    where
        T: Clone,
        U: Clone,
    {
        data.clone()
    }

    fn copy_boxed(&self, data: &Self::Box) -> Self::Box
    where
        T: Clone,
        U: Clone,
    {
        data.clone()
    }

    fn alloc(&self, data: Term<T, U>) -> Self::Box {
        Box::new(data)
    }
}

impl<T: Clone, U: Primitives<T> + Clone, A: Allocator<T, U> + Zero> Clone for Term<T, U, A> {
    fn clone(&self) -> Self {
        use Term::*;
        let alloc = A::zero();

        match self {
            Variable(index) => Term::Variable(index.clone()),
            Lambda { body, erased } => Term::Lambda {
                body: alloc.copy_boxed(body),
                erased: *erased,
            },
            Apply {
                function,
                argument,
                erased,
            } => Term::Apply {
                function: alloc.copy_boxed(function),
                argument: alloc.copy_boxed(argument),
                erased: *erased,
            },
            Put(term) => Term::Put(alloc.copy_boxed(term)),
            Duplicate { expression, body } => Term::Duplicate {
                expression: alloc.copy_boxed(expression),
                body: alloc.copy_boxed(body),
            },
            Reference(reference) => Term::Reference(reference.clone()),
            Primitive(prim) => Term::Primitive(prim.clone()),
            Term::Universe => Term::Universe,
            Term::Function {
                argument_type,
                return_type,
                erased,
            } => Term::Function {
                erased: *erased,
                argument_type: alloc.copy_boxed(argument_type),
                return_type: alloc.copy_boxed(return_type),
            },
            Term::Annotation {
                checked,
                expression,
                ty,
            } => Term::Annotation {
                checked: *checked,
                expression: alloc.copy_boxed(expression),
                ty: alloc.copy_boxed(ty),
            },
            Term::Wrap(term) => Term::Wrap(alloc.copy_boxed(term)),
        }
    }
}
