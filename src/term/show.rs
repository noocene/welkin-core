use std::fmt::{self, Display};

use super::{parse::Context, Term};

pub struct Contextualized(pub Term, pub Context);

impl Display for Contextualized {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Term::*;

        match &self.0 {
            Symbol(symbol) => write!(f, "{}", self.1.lookup(*symbol).ok_or(fmt::Error)?),
            Lambda { binding, body } => {
                write!(
                    f,
                    "\\{} {}",
                    binding,
                    Contextualized(*body.clone(), self.1.clone())
                )
            }
            Apply { function, argument } => {
                write!(
                    f,
                    "({} {})",
                    Contextualized(*function.clone(), self.1.clone()),
                    Contextualized(*argument.clone(), self.1.clone())
                )
            }
            Box(term) => write!(f, ".{}", Contextualized(*term.clone(), self.1.clone())),
            Reference(name) => write!(f, "{}", name),
            Duplicate {
                name,
                expression,
                body,
            } => {
                write!(
                    f,
                    ": {} = {} {}",
                    name,
                    Contextualized(*expression.clone(), self.1.clone()),
                    Contextualized(*body.clone(), self.1.clone())
                )
            }
        }
    }
}
