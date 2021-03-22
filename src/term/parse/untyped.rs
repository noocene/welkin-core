use combine::{many, EasyParser, Parser, Stream};
use std::str::FromStr;

use crate::term::Term;

use super::{name, term, token, Context, Errors};

fn definition<Input>(ctx: Context) -> impl Parser<Input, Output = (String, Term)>
where
    Input: Stream<Token = char>,
{
    (name().skip(token('=')), term(ctx))
}

fn definitions<Input>(ctx: Context) -> impl Parser<Input, Output = Vec<(String, Term)>>
where
    Input: Stream<Token = char>,
{
    many(definition(ctx))
}

#[derive(Clone, Default)]
pub struct Definitions {
    pub terms: Vec<(String, Term)>,
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
