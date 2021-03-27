use std::convert::Infallible;

use super::Term;

impl<T> Term<T> {
    pub fn try_map_reference<U, E, F: Fn(T) -> Result<U, E>>(self, f: F) -> Result<Term<U>, E> {
        use Term::*;

        Ok(match self {
            Variable(var) => Variable(var),
            Lambda { body, erased } => Lambda {
                body: Box::new(body.try_map_reference(f)?),
                erased,
            },
            Apply {
                function,
                argument,
                erased,
            } => Apply {
                function: Box::new(function.try_map_reference(&f)?),
                argument: Box::new(argument.try_map_reference(f)?),
                erased,
            },
            Put(term) => Put(Box::new(term.try_map_reference(f)?)),
            Duplicate { expression, body } => Duplicate {
                expression: Box::new(expression.try_map_reference(&f)?),
                body: Box::new(body.try_map_reference(f)?),
            },
            Reference(reference) => Reference(f(reference)?),
            Universe => Universe,
            Function {
                argument_type,
                return_type,
                erased,
            } => Function {
                argument_type: Box::new(argument_type.try_map_reference(&f)?),
                return_type: Box::new(return_type.try_map_reference(f)?),
                erased,
            },
            Annotation {
                checked,
                expression,
                ty,
            } => Annotation {
                expression: Box::new(expression.try_map_reference(&f)?),
                ty: Box::new(ty.try_map_reference(f)?),
                checked,
            },
            Wrap(term) => Wrap(Box::new(term.try_map_reference(f)?)),
        })
    }
    pub fn map_reference<U, F: Fn(T) -> U>(self, f: F) -> Term<U> {
        self.try_map_reference(|a| Ok::<_, Infallible>(f(a)))
            .unwrap()
    }
}
