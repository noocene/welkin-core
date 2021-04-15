use std::{borrow::Cow, collections::HashMap, fmt::Display};
use welkin_core::term::{Primitives, Term};

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
        fn ty(&self) -> Cow<'_, Term<String, Self>> {
            Cow::Owned(Term::Function {
                erased: false,
                argument_type: Box::new(Term::Universe),
                return_type: Box::new(Term::Universe),
            })
        }

        fn apply(&self, _: &Term<String, Self>) -> Term<String, Self>
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
        fn ty(&self) -> Cow<'_, Term<String, Self>> {
            Cow::Owned(Term::Function {
                erased: false,
                argument_type: Box::new(parse("Unit")),
                return_type: Box::new(parse("Unit")),
            })
        }

        fn apply(&self, term: &Term<String, Self>) -> Term<String, Self>
        where
            Self: Sized,
        {
            term.clone()
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
