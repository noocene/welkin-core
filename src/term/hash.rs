use super::{alloc::Allocator, Primitives, Show, Term};
use std::{
    hash::{Hash, Hasher},
    mem::discriminant,
};

impl<
        T: PartialEq + Hash + Show + Clone,
        V: Show + Hash + Clone + Primitives<T>,
        A: Allocator<T, V>,
    > Hash for Term<T, V, A>
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);

        match self {
            Term::Variable(variable) => variable.hash(state),
            Term::Lambda { body, erased } => {
                body.hash(state);
                erased.hash(state);
            }
            Term::Apply {
                function,
                argument,
                erased,
            } => {
                function.hash(state);
                argument.hash(state);
                erased.hash(state);
            }
            Term::Put(term) => {
                term.hash(state);
            }
            Term::Duplicate { expression, body } => {
                expression.hash(state);
                body.hash(state);
            }
            Term::Reference(reference) => {
                reference.hash(state);
            }
            Term::Primitive(prim) => {
                prim.hash(state);
            }
            Term::Universe => {}
            Term::Function {
                argument_type,
                return_type,
                erased,
            } => {
                argument_type.hash(state);
                return_type.hash(state);
                erased.hash(state);
            }
            Term::Annotation {
                checked,
                expression,
                ty,
            } => {
                checked.hash(state);
                expression.hash(state);
                ty.hash(state);
            }
            Term::Wrap(term) => {
                term.hash(state);
            }
        }
    }
}
