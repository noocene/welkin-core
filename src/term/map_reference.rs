use std::convert::Infallible;

use super::Term;

impl<T> Term<T> {
    pub fn try_map_reference<U, E, F: Fn(T) -> Result<Term<U>, E> + Clone>(
        self,
        f: F,
    ) -> Result<Term<U>, E> {
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
                function: Box::new(function.try_map_reference(f.clone())?),
                argument: Box::new(argument.try_map_reference(f)?),
                erased,
            },
            Put(term) => Put(Box::new(term.try_map_reference(f)?)),
            Duplicate { expression, body } => Duplicate {
                expression: Box::new(expression.try_map_reference(f.clone())?),
                body: Box::new(body.try_map_reference(f)?),
            },
            Reference(reference) => f(reference)?,
            Universe => Universe,
            Function {
                argument_type,
                return_type,
                erased,
            } => Function {
                argument_type: Box::new(argument_type.try_map_reference(f.clone())?),
                return_type: Box::new(return_type.try_map_reference(f)?),
                erased,
            },
            Annotation {
                checked,
                expression,
                ty,
            } => Annotation {
                expression: Box::new(expression.try_map_reference(f.clone())?),
                ty: Box::new(ty.try_map_reference(f)?),
                checked,
            },
            Wrap(term) => Wrap(Box::new(term.try_map_reference(f)?)),
        })
    }
    pub fn map_reference<U, F: Clone + Fn(T) -> Term<U>>(self, f: F) -> Term<U> {
        self.try_map_reference(|a| Ok::<_, Infallible>(f(a)))
            .unwrap()
    }
}
