use std::fmt::{self, Debug, Display};

use super::{Primitives, Term};

impl<T: Display> Show for T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

pub trait Show {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub fn debug_reference<T: Show>(data: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    Show::fmt(data, f)
}

impl<T: Show, U: Primitives<T> + Show> Term<T, U> {
    fn write(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Term::*;

        match &self {
            Variable(symbol) => write!(f, "^{}", symbol.0),
            Lambda { body, erased } => {
                write!(f, "{}x ", if *erased { "/" } else { "\\" }).and_then(|_| body.write(f))
            }
            Apply {
                function,
                argument,
                erased,
            } => write!(f, "{}", if *erased { "[" } else { "(" })
                .and_then(|_| function.write(f))
                .and_then(|_| write!(f, " "))
                .and_then(|_| argument.write(f))
                .and_then(move |_| write!(f, "{}", if *erased { "]" } else { ")" })),
            Put(term) => write!(f, ". ").and_then(|_| term.write(f)),
            Reference(name) => name.fmt(f),
            Duplicate { expression, body } => write!(f, ": X = ")
                .and_then(|_| expression.write(f))
                .and_then(|_| write!(f, " "))
                .and_then(|_| body.write(f)),
            Universe => write!(f, "*"),
            Wrap(term) => write!(f, "!").and_then(|_| term.write(f)),
            Annotation { expression, ty, .. } => write!(f, "{{ ")
                .and_then(|_| expression.write(f))
                .and_then(|_| write!(f, " : "))
                .and_then(|_| ty.write(f))
                .and_then(|_| write!(f, " }}")),
            Function {
                argument_type,
                return_type,
                erased,
            } => write!(f, "{},:", if *erased { "_" } else { "+" },)
                .and_then(|_| argument_type.write(f))
                .and_then(|_| write!(f, " "))
                .and_then(|_| return_type.write(f)),
            Primitive(prim) => prim.fmt(f),
        }
    }
}

impl<T: Show, U: Primitives<T> + Show> Debug for Term<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f)
    }
}
