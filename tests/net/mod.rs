use std::collections::HashMap;

use welkin_core::{
    net::{Index, Net, VisitNetExt},
    term::{untyped::Definitions, DefinitionResult, Definitions as Defs, Term},
};

#[cfg(feature = "accelerated")]
mod accelerated {
    use welkin_core::net::Net;

    pub fn normalize_accelerated(net: Net<u32>) -> Net<u32> {
        let mut net = net.into_accelerated().unwrap();
        net.reduce_all().unwrap();
        net.into_inner()
    }
}

#[derive(Clone)]
pub struct TestDefinitions(HashMap<String, Term<String>>);

impl Defs<String> for TestDefinitions {
    fn get(&self, name: &String) -> Option<DefinitionResult<Term<String>>> {
        self.0.get(name).map(DefinitionResult::Borrowed)
    }
}

fn round_trip(term: &str) {
    let definitions: Definitions = term.trim().parse().unwrap();
    let definitions = definitions.terms.into_iter().collect::<HashMap<_, _>>();
    let entry = definitions.get("entry").cloned().unwrap();
    let definitions = TestDefinitions(definitions);
    let entry = entry.stratified(&definitions).unwrap();
    let mut normalized = entry.clone();
    normalized.normalize().unwrap();
    let normalized = normalized.into_inner();
    let mut net = entry.clone().into_net::<Net<u32>>().unwrap();
    let mut net_recovered = net.clone().read_term(Index(0));
    net_recovered.normalize(&definitions).unwrap();
    assert!(normalized.equals(&net_recovered));
    net.reduce_all();

    let net_normalized = net.clone().read_term(Index(0));

    #[cfg(feature = "accelerated")]
    {
        let net = accelerated::normalize_accelerated(net);
        let term = net.read_term(Index(0));
        normalized.equals(&term);
    }

    assert!(normalized.equals(&net_normalized));
}

#[test]
fn trivial() {
    round_trip(
        r#"
entry = \x x
"#,
    );
}

#[test]
fn identity() {
    round_trip(
        r#"
id    = \x x
entry = (id id)
"#,
    );
}

#[test]
fn negation() {
    round_trip(
        r#"
true  = \t \f t
false = \t \f f
not   = \a \t \f (a f t)
entry = (not (not (not true)))
"#,
    );
}

#[test]
fn duplication() {
    round_trip(
        r#"
id    = \x x
entry =
  : Id = . id
  . (Id (Id Id))
"#,
    );
}

#[test]
fn epsilon() {
    round_trip(
        r#"
id    = \x \x x
entry =
  : Id = . id
  . (Id (Id Id))
"#,
    );
}

#[test]
fn unused_duplication() {
    round_trip(
        r#"
entry =
  : X = \x x
  \x x
"#,
    );
}

#[test]
fn fold() {
    round_trip(
        r#"
fold = \n \initial \call
    : Initial = initial
    : Call = call
    : F = (n . \h (Call h))
    . (F Initial)
succ = \n \f
    : F = f
    : G = (n . F)
    . \base (F (G base))
zero = \x . \x ^0
one = (succ zero)
two = (succ one)
three = (succ two)
four = (succ three)
entry = (fold one .one .\n (succ n))
        "#,
    )
}
