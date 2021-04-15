use std::convert::Infallible;

use super::{Primitives, Term};

impl<T, V: Primitives<T>> Term<T, V> {
    pub fn try_map_primitive<U: Primitives<T>, E, F: Fn(V) -> Result<U, E> + Clone>(
        self,
        f: F,
    ) -> Result<Term<T, U>, E> {
        use Term::*;

        Ok(match self {
            Variable(var) => Variable(var),
            Lambda { body, erased } => Lambda {
                body: Box::new(body.try_map_primitive(f)?),
                erased,
            },
            Primitive(primitive) => Primitive(f(primitive)?),
            Apply {
                function,
                argument,
                erased,
            } => Apply {
                function: Box::new(function.try_map_primitive(f.clone())?),
                argument: Box::new(argument.try_map_primitive(f)?),
                erased,
            },
            Put(term) => Put(Box::new(term.try_map_primitive(f)?)),
            Duplicate { expression, body } => Duplicate {
                expression: Box::new(expression.try_map_primitive(f.clone())?),
                body: Box::new(body.try_map_primitive(f)?),
            },
            Reference(reference) => Reference(reference),
            Universe => Universe,
            Function {
                argument_type,
                return_type,
                erased,
            } => Function {
                argument_type: Box::new(argument_type.try_map_primitive(f.clone())?),
                return_type: Box::new(return_type.try_map_primitive(f)?),
                erased,
            },
            Annotation {
                checked,
                expression,
                ty,
            } => Annotation {
                expression: Box::new(expression.try_map_primitive(f.clone())?),
                ty: Box::new(ty.try_map_primitive(f)?),
                checked,
            },
            Wrap(term) => Wrap(Box::new(term.try_map_primitive(f)?)),
        })
    }

    pub fn map_primitive<U: Primitives<T>, F: Clone + Fn(V) -> U>(self, f: F) -> Term<T, U> {
        self.try_map_primitive(|a| Ok::<_, Infallible>(f(a)))
            .unwrap()
    }
}
