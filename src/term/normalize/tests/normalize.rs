use crate::analysis::Empty;

use super::{assert_equivalent, parse};

#[test]
fn simple_application() {
    let mut term = parse(r#"(\x ^0 ^1)"#);
    term.normalize(&Empty).unwrap();

    assert_equivalent(term, parse(r#"^1"#));
}

#[test]
fn simple_duplication() {
    let mut term = parse(
        r#"
        : X = . ^1
        (^0 ^0)
    "#,
    );
    term.normalize(&Empty).unwrap();

    assert_equivalent(term, parse(r#"(^1 ^1)"#));
}

#[test]
fn nested_duplication() {
    let mut term = parse(
        r#"
        : X = . ^1
        : X = . ^0
        (^0 ^0)
    "#,
    );
    term.normalize(&Empty).unwrap();

    assert_equivalent(term, parse(r#"(^1 ^1)"#));
}
