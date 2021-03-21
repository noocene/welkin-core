use std::{cell::RefCell, rc::Rc, str::FromStr};

use combine::{
    many, many1, parser,
    parser::char::{alpha_num, spaces},
    token as bare_token, value, EasyParser, Parser, Stream,
};

use super::{Symbol, Term};

fn name<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
{
    spaces().with(many1(alpha_num()))
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
        name().map(move |name| ctx.resolve(&name).map(Term::Symbol).unwrap_or(Term::Reference(name)))
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
    fn duplicate[Input](ctx: Context)(Input) -> Term
        where [Input: Stream<Token = char>]
    {
        name().skip(token('=')).then(move |binding| {
            (
                value(binding),
                term(ctx.clone()).map(Box::new),
                value(ctx.clone())
            )
        }).then(|(binding, b, mut ctx): (String, _, _)| {
            (value(binding.clone()), value(b), term(ctx.with(binding)).map(Box::new))
        }).map(|(binding, expression, body)| Term::Duplicate {
            binding,
            expression,
            body,
        })
    }
}

#[derive(Debug)]
pub struct InUse;

#[derive(Clone, Default)]
pub struct Context(Rc<RefCell<Vec<String>>>);

#[derive(Clone, Default)]
pub struct Definitions {
    pub terms: Vec<(String, Term)>,
}

impl Context {
    pub(crate) fn with(&mut self, name: String) -> Self {
        self.0.borrow_mut().push(name);
        self.clone()
    }

    fn resolve(&self, name: &str) -> Option<Symbol> {
        for (idx, binding) in self.0.borrow().iter().rev().enumerate() {
            if name == binding {
                return Some(Symbol(idx));
            }
        }
        None
    }

    pub(crate) fn lookup(&self, symbol: Symbol) -> Option<String> {
        self.0.borrow().iter().rev().nth(symbol.0).cloned()
    }
}

fn term<Input>(ctx: Context) -> impl Parser<Input, Output = Term>
where
    Input: Stream<Token = char>,
{
    let parser = token('\\').with(lambda(ctx.clone()));
    let parser = parser.or(token('(').with(apply(ctx.clone())).skip(token(')')));
    let parser = parser.or(token('.').with(_box(ctx.clone())));
    let parser = parser.or(token(':').with(duplicate(ctx.clone())));
    let parser = parser.or(reference(ctx));
    spaces().with(parser)
}

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

#[derive(Debug)]
pub struct Errors {
    pub position: usize,
    pub errors: Vec<String>,
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
