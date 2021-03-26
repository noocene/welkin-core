use std::convert::TryFrom;

use crate::{
    net::{AgentType, Port, Storage},
    term::{Definitions, Stratified, Term},
    Net,
};

#[derive(Debug)]
pub enum NetError {
    TypedTerm { term: Term },
}

fn build_net<T: Storage + Clone + Eq, U: Definitions>(
    term: &Term,
    net: &mut Net<T>,
    definitions: &U,
    var_ptrs: &mut Vec<Port<T>>,
) -> Result<Port<T>, NetError> {
    use Term::*;

    Ok(match term {
        Variable(symbol) => {
            let ptr = var_ptrs.iter().rev().nth(symbol.0).unwrap().clone();
            let target = net.follow(ptr.clone());
            if target.address().is_root() || target == ptr {
                ptr
            } else {
                let duplicate = net.add(AgentType::Zeta).ports();
                net.connect(duplicate.principal, ptr);
                net.connect(duplicate.left, target);
                duplicate.right
            }
        }
        Put(term) => build_net(term, net, definitions, var_ptrs)?,
        Reference(name) => build_net(definitions.get(name).unwrap(), net, definitions, var_ptrs)?,
        Lambda { body, .. } => {
            let lambda = net.add(AgentType::Delta).ports();
            var_ptrs.push(lambda.left.clone());
            let body = build_net(body, net, definitions, var_ptrs)?;
            var_ptrs.pop();
            net.connect(lambda.right, body);
            lambda.principal
        }
        Duplicate {
            body, expression, ..
        } => {
            let expression = build_net(expression, net, definitions, var_ptrs)?;
            var_ptrs.push(expression);
            let body = build_net(body, net, definitions, var_ptrs)?;
            var_ptrs.pop();
            body
        }
        Apply { function, argument } => {
            let apply = net.add(AgentType::Delta).ports();
            let function = build_net(&function, net, definitions, var_ptrs)?;
            net.connect(apply.principal, function);
            let argument = build_net(&argument, net, definitions, var_ptrs)?;
            net.connect(apply.left, argument);
            apply.right
        }
        Annotation { expression, .. } => build_net(expression, net, definitions, var_ptrs)?,
        _ => Err(NetError::TypedTerm { term: term.clone() })?,
    })
}

impl<'a, T: Storage + Clone + Eq + Copy, U: Definitions> TryFrom<Stratified<'a, U>> for Net<T> {
    type Error = NetError;

    fn try_from(terms: Stratified<'_, U>) -> Result<Self, Self::Error> {
        let (mut net, root) = Net::new();
        let mut var_ptrs = vec![];
        let entry = build_net(&terms.0, &mut net, terms.1, &mut var_ptrs)?;
        net.connect(root, entry);
        net.bind_unbound();
        Ok(net)
    }
}
