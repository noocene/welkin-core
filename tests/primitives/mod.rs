use std::{collections::HashMap, fmt::Display};
use welkin_core::term::{alloc::Allocator, Primitives, Term};

use crate::{check, check_with, normalizes_to, parse};

#[test]
fn ty_id() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TyId;

    impl Display for TyId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(f, "tyid")
        }
    }

    impl Primitives<String> for TyId {
        fn ty<A: Allocator<String, Self>>(&self, alloc: &A) -> Term<String, Self, A> {
            Term::Function {
                erased: false,
                argument_type: alloc.alloc(Term::Universe),
                return_type: alloc.alloc(Term::Universe),
            }
        }

        fn apply<A: Allocator<String, Self>>(
            &self,
            _: &Term<String, Self, A>,
            _: &A,
        ) -> Term<String, Self, A>
        where
            Self: Sized,
        {
            todo!()
        }
    }

    check(
        parse(
            r#"
            *
    "#,
        ),
        Term::Apply {
            function: Box::new(Term::Primitive(TyId)),
            argument: Box::new(Term::Universe),
            erased: false,
        },
    );
}

#[test]
fn unit_id() {
    let mut definitions = HashMap::new();
    definitions.insert(
        "Unit".into(),
        (
            parse("*"),
            parse("_unit,prop: +,:Unit * +,:(prop /prop \\x x) (prop unit)"),
        ),
    );

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct UnitId;

    impl Display for UnitId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(f, "idid")
        }
    }

    impl Primitives<String> for UnitId {
        fn ty<A: Allocator<String, Self>>(&self, alloc: &A) -> Term<String, Self, A> {
            Term::Function {
                erased: false,
                argument_type: alloc.alloc(Term::Reference("Unit".into())),
                return_type: alloc.alloc(Term::Reference("Unit".into())),
            }
        }

        fn apply<A: Allocator<String, Self>>(
            &self,
            term: &Term<String, Self, A>,
            alloc: &A,
        ) -> Term<String, Self, A>
        where
            Self: Sized,
        {
            alloc.copy(term)
        }
    }

    let unit = parse::<UnitId>(r#" /prop \x x "#);

    let term = Term::Apply {
        function: Box::new(Term::Primitive(UnitId)),
        argument: Box::new(unit.clone()),
        erased: false,
    };

    check_with(parse("Unit"), term.clone(), &definitions);

    normalizes_to(term, unit, &definitions);
}
