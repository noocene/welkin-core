use crate::term::{Index, Term};

use super::{assert_equivalent, parse};

#[test]
fn trivial() {
    let mut term = parse("^0");
    let other = parse(r#"\x ^2"#);

    term.substitute(Index::top(), &other);

    assert_equivalent(term, other);

    let mut term = parse("(^0 ^1)");
    let other = parse(r#"\x ^2"#);

    term.substitute(Index::top().child(), &other);

    assert_equivalent(term, parse(r#"(^0 \x ^2)"#));
}

#[test]
fn multiple() {
    let mut term = parse("(^0 ^1)");
    let other = parse(r#"\x ^0"#);

    for _ in 0..2 {
        term.substitute(Index::top(), &other);
    }

    assert_equivalent(term, parse(r#"(\x ^0 \x ^0)"#));
}

#[test]
fn in_lambda() {
    let mut term = parse(r#"(\x ^1 ^0)"#);
    let other = parse(r#"\x ^0"#);

    term.substitute(Index::top(), &other);

    assert_equivalent(term, parse(r#"(\x ^0 \x ^0)"#));
}

#[test]
fn in_function() {
    let mut term: Term<String> = parse(r#"+,:* (^3 ^2)"#);

    term.substitute_function(parse("^2"), &parse("^2"));

    panic!("{:?}", term);
}
