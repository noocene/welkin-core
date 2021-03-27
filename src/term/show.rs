use std::fmt::{self, Debug, Display};

use super::{parse::Context, Term};

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

impl<T: Show> Term<T> {
    fn write(&self, f: &mut fmt::Formatter<'_>, ctx: &mut Context) -> fmt::Result {
        use Term::*;

        match &self {
            Variable(symbol) => write!(
                f,
                "{}",
                ctx.lookup(*symbol)
                    .unwrap_or_else(|| { format!("^{}", symbol.0) })
            ),
            Lambda {
                binding,
                body,
                erased,
            } => write!(f, "{}{} ", if *erased { "/" } else { "\\" }, binding)
                .and_then(|_| body.write(f, &mut ctx.with(binding.clone()))),
            Apply {
                function,
                argument,
                erased,
            } => write!(f, "{}", if *erased { "[" } else { "(" })
                .and_then(|_| function.write(f, ctx))
                .and_then(|_| write!(f, " "))
                .and_then(|_| argument.write(f, ctx))
                .and_then(move |_| write!(f, "{}", if *erased { "]" } else { ")" })),
            Put(term) => write!(f, ". ").and_then(|_| term.write(f, ctx)),
            Reference(name) => name.fmt(f),
            Duplicate {
                binding,
                expression,
                body,
            } => write!(f, ": {} = ", binding)
                .and_then(|_| expression.write(f, ctx))
                .and_then(|_| write!(f, " "))
                .and_then(|_| body.write(f, &mut ctx.with(binding.clone()))),
            Universe => write!(f, "*"),
            Wrap(term) => write!(f, "!").and_then(|_| term.write(f, ctx)),
            Annotation { expression, ty, .. } => write!(f, "{{ ")
                .and_then(|_| expression.write(f, ctx))
                .and_then(|_| write!(f, " : "))
                .and_then(|_| ty.write(f, ctx))
                .and_then(|_| write!(f, " }}")),
            Function {
                self_binding,
                argument_binding,
                argument_type,
                return_type,
                erased,
            } => write!(
                f,
                "{}{},{}:",
                if *erased { "_" } else { "+" },
                self_binding,
                argument_binding
            )
            .and_then(|_| argument_type.write(f, ctx))
            .and_then(|_| write!(f, " "))
            .and_then(|_| {
                let mut ctx = ctx
                    .with(self_binding.clone())
                    .with(argument_binding.clone());
                return_type.write(f, &mut ctx)
            }),
        }
    }
}

impl<T: Show> Debug for Term<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut context = Default::default();
        self.write(f, &mut context)
    }
}
