use std::collections::HashMap;

use welkin_core::term::{typed::Definitions, Primitives, Show, Term, TypedDefinitions};

mod net;
mod primitives;

#[allow(dead_code)]
fn check_all(terms: &str) {
    let definitions: Definitions = terms.trim().parse().unwrap();

    let definitions: HashMap<_, _> = definitions.terms.into_iter().collect();
    for (_, def) in &definitions {
        def.1.is_stratified(&definitions).unwrap();
        def.0.check(&Term::Universe, &definitions).unwrap();
        def.1.check(&def.0, &definitions).unwrap();
    }
}

#[track_caller]
fn parse<V: Primitives<String>>(term: &str) -> Term<String, V> {
    let term: Term<String> = term.trim().parse().unwrap();
    term.map_primitive(|_| panic!())
}

fn check<V: Primitives<String> + Clone>(ty: Term<String, V>, term: Term<String, V>)
where
    Term<String, V>: PartialEq,
    V: Show,
{
    let definitions = HashMap::new();

    // TODO stratification check

    ty.check(&Term::Universe, &definitions).unwrap();
    term.check(&ty, &definitions).unwrap();
}

fn check_with<V: Primitives<String> + Clone, D: TypedDefinitions<String, V>>(
    ty: Term<String, V>,
    term: Term<String, V>,
    definitions: &D,
) where
    Term<String, V>: PartialEq,
    V: Show,
{
    // TODO stratification check

    ty.check(&Term::Universe, definitions).unwrap();
    term.check(&ty, definitions).unwrap();
}

fn normalizes_to<V: Primitives<String> + Clone, D: TypedDefinitions<String, V>>(
    mut term: Term<String, V>,
    target: Term<String, V>,
    definitions: &D,
) where
    Term<String, V>: PartialEq,
    V: Show,
{
    term.normalize(definitions).unwrap();
    assert_eq!(term, target);
}
