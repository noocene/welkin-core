use combine::{many, EasyParser, Parser, Stream};
use std::str::FromStr;

type Term = crate::term::Term<String>;

use super::{name, term, token, Context, ParseError};

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
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s
            .split('\n')
            .filter(|line| !line.starts_with("-") && !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let mut position = None;
        let ctx: Context = Default::default();
        let data = definitions(ctx.clone())
            .easy_parse(s.as_str())
            .map_err(|e| {
                position = Some(e.position);
                ParseError::from(e)
            })
            .and_then(|(terms, remainder)| {
                if !remainder.is_empty() {
                    Err(ParseError {
                        got: format!("{:?}", remainder),
                        expected: vec!["end of input".into()],
                        position: s.len(),
                    })
                } else {
                    Ok(Definitions { terms })
                }
            })
            .map_err(|mut e| {
                if let Some(position) = position {
                    e.position = position.translate_position(&s);
                };
                e
            });

        data
    }
}
