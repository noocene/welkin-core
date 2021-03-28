use std::{
    fmt::{self, Debug, Display},
    rc::Rc,
    str::FromStr,
};
use thiserror::Error;

pub mod typed;
pub mod untyped;

use combine::{
    easy::{Error, Errors, Info},
    many, many1, parser,
    parser::char::{alpha_num, digit, spaces},
    stream::PointerOffset,
    token as bare_token, value, EasyParser, Parser, Stream,
};

use super::Index;

type Term = super::Term<String>;

fn name<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
{
    spaces().with(many1(alpha_num()))
}

fn maybe_name<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
{
    spaces().with(many(alpha_num()))
}

fn token<Input>(token: char) -> impl Parser<Input, Output = char>
where
    Input: Stream<Token = char>,
{
    spaces().with(bare_token(token))
}

fn variable<Input>() -> impl Parser<Input, Output = Term>
where
    Input: Stream<Token = char>,
{
    spaces()
        .with(many1(digit()))
        .map(|string: String| Term::Variable(Index(string.parse::<usize>().unwrap())))
}

parser! {
    fn lambda[Input](erased: bool, ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        let erased = *erased;
        name().then(|name| term(ctx.with(name)).map(Box::new)).map(move |body| Term::Lambda { erased, body })
    }
}

parser! {
    fn apply[Input](erased: bool, ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        let erased = *erased;
        (term(ctx.clone()).map(Box::new), many1(term(ctx.clone()))).map(move |(function, arguments): (_, Vec<_>)| {
            let mut arguments = arguments.into_iter();
            let mut term = Term::Apply {
                function,
                erased,
                argument: Box::new(arguments.next().unwrap()),
            };
            while let Some(argument) = arguments.next() {
                term = Term::Apply {
                    function: Box::new(term),
                    erased,
                    argument: Box::new(argument),
                };
            }
            term
        })
    }
}

parser! {
    fn reference[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        name().map(move |name| ctx.resolve(&name).map(Term::Variable).unwrap_or(Term::Reference(name)))
    }
}

parser! {
    fn _box[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        term(ctx.clone()).map(Box::new).map(Term::Put)
    }
}

parser! {
    fn wrap[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        term(ctx.clone()).map(Box::new).map(Term::Wrap)
    }
}

parser! {
    fn duplicate[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        name().skip(token('=')).then(move |binding| {
            (
                value(binding),
                term(ctx.clone()).map(Box::new),
                value(ctx.clone())
            )
        }).then(|(binding, b, ctx): (String, _, _)| {
            (value(b), term(ctx.with(binding)).map(Box::new))
        }).map(|(expression, body)| Term::Duplicate {
            expression,
            body,
        })
    }
}

parser! {
    fn annotation[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        (term(ctx.clone()).skip(token(':')).map(Box::new), term(ctx.clone()).map(Box::new)).map(|(expression, ty)| {
            Term::Annotation {
                expression,
                ty,
                checked: false
            }
        })
    }
}

#[derive(Debug)]
pub struct InUse;

#[derive(Clone, Default, Debug)]
pub struct Context(Rc<Vec<String>>);

impl Context {
    pub(crate) fn with(&self, name: String) -> Self {
        let mut data = (*self.0).clone();
        data.push(name);
        Context(Rc::new(data))
    }

    pub(crate) fn resolve(&self, name: &str) -> Option<Index> {
        for (idx, binding) in self.0.iter().rev().enumerate() {
            if name == binding {
                return Some(Index(idx));
            }
        }
        None
    }
}

parser! {
    fn function[Input](erased: bool, ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        let erased = *erased;

        (
            maybe_name().skip(token(',')),
            maybe_name().skip(token(':')),
            term(ctx.clone()).map(Box::new),
        )
        .then({
            let ctx = ctx.clone();
            move |(self_binding, argument_binding, argument_type)| {
                let ctx = ctx.with(self_binding.clone()).with(argument_binding.clone());
                (value(argument_type), term(ctx).map(Box::new))
            }
        })
        .map(move |(argument_type, return_type)| {
            Term::Function {
                argument_type,
                return_type,
                erased
            }
        })
    }
}

pub fn term<Input>(ctx: Context) -> impl Parser<Input, Output = Term>
where
    Input: Stream<Token = char>,
{
    let parser = token('\\').with(lambda(false, ctx.clone()));
    let parser = parser.or(token('/').with(lambda(true, ctx.clone())));
    let parser = parser.or(token('(').with(apply(false, ctx.clone())).skip(token(')')));
    let parser = parser.or(token('[').with(apply(true, ctx.clone())).skip(token(']')));
    let parser = parser.or(token('{').with(annotation(ctx.clone())).skip(token('}')));
    let parser = parser.or(token('.').with(_box(ctx.clone())));
    let parser = parser.or(token(':').with(duplicate(ctx.clone())));
    let parser = parser.or(token('+').with(function(false, ctx.clone())));
    let parser = parser.or(token('_').with(function(true, ctx.clone())));
    let parser = parser.or(token('*').with(value(Term::Universe)));
    let parser = parser.or(token('!').with(wrap(ctx.clone())));
    let parser = parser.or(token('^').with(variable()));
    let parser = parser.or(reference(ctx));
    spaces().with(parser)
}

pub fn parse<Input>() -> impl Parser<Input, Output = Term>
where
    Input: Stream<Token = char>,
{
    let ctx = Default::default();
    term(ctx)
}

#[derive(Debug, Error)]
pub struct ParseError {
    got: String,
    expected: Vec<String>,
    position: usize,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unexpected {} at position {}", self.got, self.position)?;
        if !self.expected.is_empty() {
            write!(
                f,
                "\nExpected {}",
                if self.expected.len() > 1 {
                    "one of "
                } else {
                    ""
                },
            )?;
            let mut iter = self.expected.iter().peekable();
            while let Some(expected) = iter.next() {
                if self.expected.len() > 1 && !iter.peek().is_some() {
                    write!(f, "or ")?;
                }
                write!(f, "{}", expected)?;
                if iter.peek().is_some() {
                    write!(f, ", ")?;
                }
            }
        }

        Ok(())
    }
}

impl<T: Debug, R: Debug, P: ?Sized> From<Errors<T, R, PointerOffset<P>>> for ParseError {
    fn from(e: Errors<T, R, PointerOffset<P>>) -> Self {
        ParseError {
            position: e.position.0,
            got: e
                .errors
                .iter()
                .find_map(|e| match e {
                    Error::Unexpected(e) => Some(match e {
                        Info::Token(token) => format!("token {:?}", token),
                        _ => format!("{:?}", e),
                    }),
                    _ => None,
                })
                .unwrap(),
            expected: {
                let mut expected: Vec<String> = e
                    .errors
                    .iter()
                    .filter_map(|e| match e {
                        Error::Expected(e) => Some(match e {
                            Info::Token(token) => format!("{:?}", token),
                            Info::Static(stat) => (*stat).to_owned(),
                            _ => format!("{:?}", e),
                        }),
                        _ => None,
                    })
                    .collect();
                expected.sort();
                expected
            },
        }
    }
}

impl FromStr for Term {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let definitions = Default::default();
        let mut position = None;
        let data = term(definitions)
            .easy_parse(s)
            .map_err(|e| {
                position = Some(e.position);
                ParseError::from(e)
            })
            .and_then(|(a, remainder)| {
                if !remainder.is_empty() {
                    Err(ParseError {
                        got: format!("{:?}", remainder),
                        expected: vec!["end of input".into()],
                        position: s.len(),
                    })
                } else {
                    Ok(a)
                }
            })
            .map_err(|mut e| {
                if let Some(position) = position {
                    e.position = position.translate_position(s);
                }
                e
            });

        data
    }
}
