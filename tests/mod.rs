use std::{collections::HashMap, hash::Hash};

use welkin_core::term::{typed::Definitions, NullCache, Primitives, Show, Term, TypedDefinitions};

mod net;
mod primitives;

#[allow(dead_code)]
fn check_all(terms: &str) {
    let definitions: Definitions = terms.trim().parse().unwrap();

    let definitions: HashMap<_, _> = definitions.terms.into_iter().collect();
    for (_, def) in &definitions {
        def.1.is_stratified().unwrap();
        def.0
            .check(&Term::Universe, &definitions, &mut NullCache)
            .unwrap();
        def.1.check(&def.0, &definitions, &mut NullCache).unwrap();
    }
}

#[track_caller]
fn parse<V: Primitives<String>>(term: &str) -> Term<String, V> {
    let term: Term<String> = term.trim().parse().unwrap();
    term.map_primitive(|_| panic!())
}

fn check<V: Primitives<String> + Clone + Hash>(ty: Term<String, V>, term: Term<String, V>)
where
    V: Show,
{
    let definitions = HashMap::new();

    // TODO stratification check

    ty.check(&Term::Universe, &definitions, &mut NullCache)
        .unwrap();
    term.check(&ty, &definitions, &mut NullCache).unwrap();
}

fn check_with<V: Primitives<String> + Clone + Hash, D: TypedDefinitions<String, V>>(
    ty: Term<String, V>,
    term: Term<String, V>,
    definitions: &D,
) where
    V: Show,
{
    // TODO stratification check

    ty.check(&Term::Universe, definitions, &mut NullCache)
        .unwrap();
    term.check(&ty, definitions, &mut NullCache).unwrap();
}

fn normalizes_to<V: Primitives<String> + Clone + Hash, D: TypedDefinitions<String, V>>(
    mut term: Term<String, V>,
    target: Term<String, V>,
    definitions: &D,
) where
    V: Show,
{
    term.normalize(definitions).unwrap();
    assert!(term
        .equivalent(&target, definitions, &mut NullCache)
        .unwrap());
}
