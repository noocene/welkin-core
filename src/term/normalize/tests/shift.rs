use super::{assert_equivalent, parse};
use crate::term::Index;

#[test]
fn bare_variable() {
    let mut term = parse("^0");
    term.shift(Index::top());
    assert_equivalent(term, parse("^1"));

    let mut term = parse("^0");
    term.shift(Index::top().child());
    assert_equivalent(term, parse("^0"));

    let mut term = parse("^1");
    term.shift(Index::top().child().child());
    assert_equivalent(term, parse("^1"));
}

#[test]
fn in_lambda() {
    let mut term = parse(r#"\x ^0"#);
    term.shift(Index::top());
    assert_equivalent(term, parse(r#"\x ^0"#));

    let mut term = parse(r#"\x ^1"#);
    term.shift(Index::top());
    assert_equivalent(term, parse(r#"\x ^2"#));
}
