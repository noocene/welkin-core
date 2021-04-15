use std::{collections::HashMap, fmt::Debug, fs::read_to_string, io, process::exit};
#[cfg(any(feature = "graphviz", feature = "accelerated"))]
use welkin_core::net::{Index, Net, VisitNetExt};
use welkin_core::term::{typed::Definitions, ParseError, Term};

fn e<E: Debug>(e: E) -> String {
    format!("{:?}", e)
}

fn entry(buffer: String, term: String) -> Result<(), String> {
    let definitions: Definitions = buffer.parse().map_err(|e: ParseError| e.to_string())?;

    let definitions: HashMap<_, _> = definitions.terms.into_iter().collect();
    for (_, def) in &definitions {
        def.1.is_stratified(&definitions).map_err(e)?;
        def.0.check(&Term::Universe, &definitions).map_err(e)?;
        def.1.check(&def.0, &definitions).map_err(e)?;
    }

    let entry = Term::Reference(term).stratified(&definitions).map_err(e)?;

    #[cfg(any(feature = "graphviz", feature = "accelerated"))]
    let entry = entry.into_net::<Net<u32>>().unwrap();

    #[cfg(feature = "accelerated")]
    let entry = {
        let mut entry = entry.into_accelerated().unwrap();
        println!("DONE in {} rewrites", entry.reduce_all().unwrap());
        entry.into_inner()
    };

    #[cfg(all(not(feature = "accelerated"), feature = "graphviz"))]
    let entry = {
        let mut entry = entry;
        println!("DONE in {} rewrites", entry.reduce_all());
        entry
    };

    #[cfg(any(feature = "graphviz", feature = "accelerated"))]
    let term: Term<String> = {
        #[cfg(feature = "graphviz")]
        {
            entry
                .render_to(&mut std::fs::File::create("example1.dot").unwrap())
                .unwrap();
        }
        entry.read_term(entry.get(Index(0)).ports().principal)
    };

    #[cfg(not(any(feature = "graphviz", feature = "accelerated")))]
    let term = {
        let mut entry = entry;
        entry.normalize().unwrap();
        entry.into_inner()
    };

    println!("{:?}", term);

    Ok(())
}

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);

    if let (Some(file), Some(term)) = (args.next(), args.next()) {
        let buffer = read_to_string(file)?;
        if let Err(e) = entry(buffer, term) {
            eprintln!("{}", e);
            exit(1);
        }
    } else {
        eprintln!(
            r#"Usage: welkin-core <FILE> <TERM>

Typecheck FILE as welkin-core definitions and print the normalization of TERM"#
        )
    }

    Ok(())
}
