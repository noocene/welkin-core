use combine::{many, EasyParser, Parser, Stream};
use std::str::FromStr;

use crate::term::Term;

use super::{name, term, token, Context, ParseError, Referent};

fn definition<'a, Input: 'a, T: Referent<Input> + 'a>(
    ctx: Context,
) -> impl Parser<Input, Output = (String, Term<T>)> + 'a
where
    Input: Stream<Token = char>,
{
    (name().skip(token('=')), term(ctx))
}

fn definitions<'a, Input: 'a, T: Referent<Input> + 'a>(
    ctx: Context,
) -> impl Parser<Input, Output = Vec<(String, Term<T>)>> + 'a
where
    Input: Stream<Token = char>,
{
    many(definition(ctx))
}

#[derive(Clone, Default)]
pub struct Definitions<T = String> {
    pub terms: Vec<(String, Term<T>)>,
}

impl<T> FromStr for Definitions<T>
where
    for<'a> T: Referent<combine::easy::Stream<&'a str>>,
{
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
