use std::fmt::{self, Debug};

use super::{parse::Context, Term};

impl Term {
    fn write(&self, f: &mut fmt::Formatter<'_>, ctx: &mut Context) -> fmt::Result {
        use Term::*;

        match &self {
            Variable(symbol) => write!(
                f,
                "{}",
                ctx.lookup(*symbol)
                    .unwrap_or_else(|| { format!("^{}", symbol.0) })
            ),
            Lambda { binding, body } => write!(f, "\\{} ", binding)
                .and_then(|_| body.write(f, &mut ctx.with(binding.clone()))),
            Apply { function, argument } => write!(f, "(")
                .and_then(|_| function.write(f, ctx))
                .and_then(|_| write!(f, " "))
                .and_then(|_| argument.write(f, ctx))
                .and_then(|_| write!(f, ")")),
            Put(term) => write!(f, ". ").and_then(|_| term.write(f, ctx)),
            Reference(name) => write!(f, "{}", name),
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
            } => {
                let mut ctx = ctx
                    .with(self_binding.clone())
                    .with(argument_binding.clone());
                write!(f, "+{},{}:", self_binding, argument_binding)
                    .and_then(|_| argument_type.write(f, &mut ctx))
                    .and_then(|_| write!(f, " "))
                    .and_then(|_| return_type.write(f, &mut ctx))
            }
        }
    }
}

impl Debug for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut context = Default::default();
        self.write(f, &mut context)
    }
}
