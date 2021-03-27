use std::convert::Infallible;

use super::Term;

impl<T> Term<T> {
    pub fn try_map_reference<U, E, F: Fn(T) -> Result<U, E>>(self, f: F) -> Result<Term<U>, E> {
        Ok(match self {
            Term::Variable(var) => Term::Variable(var),
            Term::Lambda { body, erased } => Term::Lambda {
                body: Box::new(body.try_map_reference(f)?),
                erased,
            },
            Term::Apply {
                function,
                argument,
                erased,
            } => Term::Apply {
                function: Box::new(function.try_map_reference(&f)?),
                argument: Box::new(argument.try_map_reference(f)?),
                erased,
            },
            Term::Put(term) => Term::Put(Box::new(term.try_map_reference(f)?)),
            Term::Duplicate { expression, body } => Term::Duplicate {
                expression: Box::new(expression.try_map_reference(&f)?),
                body: Box::new(body.try_map_reference(f)?),
            },
            Term::Reference(reference) => Term::Reference(f(reference)?),
            Term::Universe => Term::Universe,
            Term::Function {
                argument_type,
                return_type,
                erased,
            } => Term::Function {
                argument_type: Box::new(argument_type.try_map_reference(&f)?),
                return_type: Box::new(return_type.try_map_reference(f)?),
                erased,
            },
            Term::Annotation {
                checked,
                expression,
                ty,
            } => Term::Annotation {
                expression: Box::new(expression.try_map_reference(&f)?),
                ty: Box::new(ty.try_map_reference(f)?),
                checked,
            },
            Term::Wrap(term) => Term::Wrap(Box::new(term.try_map_reference(f)?)),
        })
    }
    pub fn map_reference<U, F: Fn(T) -> U>(self, f: F) -> Term<U> {
        self.try_map_reference(|a| Ok::<_, Infallible>(f(a)))
            .unwrap()
    }
}
