use crate::{
    net::{AgentType, Port, Storage},
    term::{Definitions, Stratified, Term},
    Net,
};

fn build_net<T: Storage + Clone + Eq, U: Definitions>(
    term: &Term,
    net: &mut Net<T>,
    level: usize,
    definitions: &U,
    var_ptrs: &mut Vec<Port<T>>,
) -> Port<T> {
    use Term::*;

    match term {
        Symbol(symbol) => {
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
        Put(term) => build_net(term, net, level + 1, definitions, var_ptrs),
        Reference(name) => build_net(
            definitions.get(name).unwrap(),
            net,
            level,
            definitions,
            var_ptrs,
        ),
        Lambda { body, .. } => {
            let lambda = net.add(AgentType::Delta).ports();
            var_ptrs.push(lambda.left.clone());
            let body = build_net(body, net, level, definitions, var_ptrs);
            var_ptrs.pop();
            net.connect(lambda.right, body);
            lambda.principal
        }
        Duplicate {
            body, expression, ..
        } => {
            let expression = build_net(expression, net, level, definitions, var_ptrs);
            var_ptrs.push(expression);
            let body = build_net(body, net, level, definitions, var_ptrs);
            var_ptrs.pop();
            body
        }
        Apply { function, argument } => {
            let apply = net.add(AgentType::Delta).ports();
            let function = build_net(function, net, level, definitions, var_ptrs);
            net.connect(apply.principal, function);
            let argument = build_net(argument, net, level, definitions, var_ptrs);
            net.connect(apply.left, argument);
            apply.right
        }
        _ => panic!("cannot translate typed net!"),
    }
}

impl<'a, T: Storage + Clone + Eq + Copy, U: Definitions> From<Stratified<'a, U>> for Net<T> {
    fn from(terms: Stratified<'_, U>) -> Self {
        let (mut net, root) = Net::new();
        let mut var_ptrs = vec![];
        let entry = build_net(&terms.0, &mut net, 0, terms.1, &mut var_ptrs);
        net.connect(root, entry);
        net.bind_unbound();
        net
    }
}
