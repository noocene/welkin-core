use crate::{
    analysis::Empty,
    term::{NullCache, Primitives, Term},
};

mod normalize;
mod shift;
mod substitute;

#[track_caller]
fn parse<V: Primitives<String>>(term: &str) -> Term<String, V> {
    let term: Term<String> = term.trim().parse().unwrap();
    term.map_primitive(|_| panic!())
}

#[track_caller]
fn assert_equivalent(a: Term<String>, b: Term<String>) {
    if !a.equivalent(&b, &Empty, &mut NullCache).unwrap() {
        panic!("assertion failed: {:?} != {:?}", a, b);
    }
}
