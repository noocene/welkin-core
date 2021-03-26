use std::convert::TryFrom;

use crate::{
    net::{AgentType, Port, Storage},
    term::{Definitions, Index, Stratified, Term},
    Net,
};

#[derive(Debug)]
pub enum NetError {
    TypedTerm { term: Term },
}

impl Term {
    fn build_net<T: Storage + Clone + Eq, U: Definitions>(
        &self,
        net: &mut Net<T>,
        definitions: &U,
        var_ptrs: &mut Vec<Port<T>>,
    ) -> Result<Port<T>, NetError> {
        use Term::*;

        Ok(match self {
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
            Put(term) => term.build_net(net, definitions, var_ptrs)?,
            Reference(name) => {
                definitions
                    .get(name)
                    .unwrap()
                    .build_net(net, definitions, var_ptrs)?
            }
            Lambda { body, erased, .. } => {
                if *erased {
                    let mut body = body.clone();
                    body.substitute_top(&Term::Variable(Index::top()));
                    body.build_net(net, definitions, var_ptrs)?
                } else {
                    let lambda = net.add(AgentType::Delta).ports();
                    var_ptrs.push(lambda.left.clone());
                    let body = body.build_net(net, definitions, var_ptrs)?;
                    var_ptrs.pop();
                    net.connect(lambda.right, body);
                    lambda.principal
                }
            }
            Duplicate {
                body, expression, ..
            } => {
                let expression = expression.build_net(net, definitions, var_ptrs)?;
                var_ptrs.push(expression);
                let body = body.build_net(net, definitions, var_ptrs)?;
                var_ptrs.pop();
                body
            }
            Apply {
                function,
                argument,
                erased,
                ..
            } => {
                if *erased {
                    function.build_net(net, definitions, var_ptrs)?
                } else {
                    let apply = net.add(AgentType::Delta).ports();
                    let function = function.build_net(net, definitions, var_ptrs)?;
                    net.connect(apply.principal, function);
                    let argument = argument.build_net(net, definitions, var_ptrs)?;
                    net.connect(apply.left, argument);
                    apply.right
                }
            }
            Annotation { expression, .. } => expression.build_net(net, definitions, var_ptrs)?,
            _ => Err(NetError::TypedTerm { term: self.clone() })?,
        })
    }
}

impl<'a, T: Storage + Clone + Eq + Copy, U: Definitions> TryFrom<Stratified<'a, U>> for Net<T> {
    type Error = NetError;

    fn try_from(terms: Stratified<'_, U>) -> Result<Self, Self::Error> {
        let (mut net, root) = Net::new();
        let mut var_ptrs = vec![];
        let entry = terms.0.build_net(&mut net, terms.1, &mut var_ptrs)?;
        net.connect(root, entry);
        net.bind_unbound();
        Ok(net)
    }
}
