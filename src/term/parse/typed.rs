use combine::{many, EasyParser, Parser, Stream};
use std::str::FromStr;

use crate::term::Term;

use super::{name, term, token, untyped, Context, Errors};

fn definition<Input>(ctx: Context) -> impl Parser<Input, Output = (String, (Term, Term))>
where
    Input: Stream<Token = char>,
{
    (
        name().skip(token(':')),
        term(ctx.clone()).skip(token('=')),
        term(ctx),
    )
        .map(|(a, b, c)| (a, (b, c)))
}

fn definitions<Input>(ctx: Context) -> impl Parser<Input, Output = Vec<(String, (Term, Term))>>
where
    Input: Stream<Token = char>,
{
    many(definition(ctx))
}

#[derive(Clone, Default)]
pub struct Definitions {
    pub terms: Vec<(String, (Term, Term))>,
}

impl Definitions {
    pub fn untyped(&self) -> untyped::Definitions {
        untyped::Definitions {
            terms: self
                .terms
                .clone()
                .into_iter()
                .map(|(a, (_, c))| (a, c))
                .collect(),
        }
    }
}

impl FromStr for Definitions {
    type Err = Errors;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s
            .split('\n')
            .filter(|line| !line.starts_with("-") && !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let ctx: Context = Default::default();
        let data = definitions(ctx.clone())
            .easy_parse(s.as_str())
            .map_err(|e| Errors {
                position: e.position.translate_position(&s),
                errors: e.errors.into_iter().map(|a| format!("{}", a)).collect(),
            })
            .and_then(|(terms, remainder)| {
                if !remainder.is_empty() {
                    Err(Errors {
                        position: s.len() - 1,
                        errors: vec![format!(
                            "parsing finished with {} chars left over: {:?}",
                            remainder.len(),
                            remainder
                        )],
                    })
                } else {
                    Ok(Definitions { terms })
                }
            });

        data
    }
}
