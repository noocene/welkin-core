use std::{rc::Rc, str::FromStr};

pub mod typed;
pub mod untyped;

use combine::{
    many, many1, parser,
    parser::char::{alpha_num, spaces},
    token as bare_token, value, EasyParser, Parser, Stream,
};

use super::{Index, Term};

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

parser! {
    fn lambda[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        name().then(|name| (value(name.clone()), term(ctx.with(name)).map(Box::new))).map(|(binding, body)| Term::Lambda { binding, body })
    }
}

parser! {
    fn apply[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        (term(ctx.clone()).map(Box::new), many1(term(ctx.clone()))).map(|(function, arguments): (_, Vec<_>)| {
            let mut arguments = arguments.into_iter();
            let mut term = Term::Apply {
                function,
                argument: Box::new(arguments.next().unwrap()),
            };
            while let Some(argument) = arguments.next() {
                term = Term::Apply {
                    function: Box::new(term),
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
            (value(binding.clone()), value(b), term(ctx.with(binding)).map(Box::new))
        }).map(|(binding, expression, body)| Term::Duplicate {
            binding,
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

    pub(crate) fn lookup(&self, symbol: Index) -> Option<String> {
        self.0.iter().rev().nth(symbol.0).cloned()
    }
}

parser! {
    fn function[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        (
            maybe_name().skip(token(',')),
            maybe_name().skip(token(':')),
            term(ctx.clone()).map(Box::new),
        )
        .then({
            let ctx = ctx.clone();
            move |(self_binding, argument_binding, argument_type)| {
                let ctx = ctx.with(self_binding.clone()).with(argument_binding.clone());
                (value(self_binding), value(argument_binding), value(argument_type), term(ctx).map(Box::new))
            }
        })
        .map(|(self_binding, argument_binding, argument_type, return_type)| {
            Term::Function {
                self_binding,
                argument_binding,
                argument_type,
                return_type
            }
        })
    }
}

fn term<Input>(ctx: Context) -> impl Parser<Input, Output = Term>
where
    Input: Stream<Token = char>,
{
    let parser = token('\\').with(lambda(ctx.clone()));
    let parser = parser.or(token('(').with(apply(ctx.clone())).skip(token(')')));
    let parser = parser.or(token('{').with(annotation(ctx.clone())).skip(token('}')));
    let parser = parser.or(token('.').with(_box(ctx.clone())));
    let parser = parser.or(token(':').with(duplicate(ctx.clone())));
    let parser = parser.or(token('+').with(function(ctx.clone())));
    let parser = parser.or(token('*').with(value(Term::Universe)));
    let parser = parser.or(token('!').with(wrap(ctx.clone())));
    let parser = parser.or(reference(ctx));
    spaces().with(parser)
}

#[derive(Debug)]
pub struct Errors {
    pub position: usize,
    pub errors: Vec<String>,
}

impl FromStr for Term {
    type Err = Errors;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let definitions = Default::default();
        let data = term(definitions)
            .easy_parse(s)
            .map_err(|e| Errors {
                position: e.position.translate_position(&s),
                errors: e.errors.into_iter().map(|a| format!("{}", a)).collect(),
            })
            .and_then(|(a, remainder)| {
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
                    Ok(a)
                }
            });

        data
    }
}
