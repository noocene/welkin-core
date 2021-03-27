use std::convert::TryFrom;

use derivative::Derivative;

use crate::{
    net::{AgentType, Net, Port, Storage},
    term::{Definitions, Index, Show, Stratified, Term},
};

#[derive(Derivative)]
#[derivative(Debug(bound = "T: Show"))]
pub enum NetError<T> {
    TypedTerm(Term<T>),
}

impl<T> Term<T> {
    fn build_net<S: Storage + Clone + Eq, U: Definitions<T>>(
        &self,
        net: &mut Net<S>,
        definitions: &U,
        var_ptrs: &mut Vec<Port<S>>,
    ) -> Result<Port<S>, NetError<T>>
    where
        T: Clone,
    {
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
            Lambda { body, erased } => {
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
            _ => Err(NetError::TypedTerm(self.clone()))?,
        })
    }
}

impl<'a, S: Storage + Clone + Eq + Copy, T: Clone, U: Definitions<T>> TryFrom<Stratified<'a, T, U>>
    for Net<S>
{
    type Error = NetError<T>;

    fn try_from(terms: Stratified<'_, T, U>) -> Result<Self, Self::Error> {
        let (mut net, root) = Net::new();
        let mut var_ptrs = vec![];
        let entry = terms.0.build_net(&mut net, terms.1, &mut var_ptrs)?;
        net.connect(root, entry);
        net.bind_unbound();
        Ok(net)
    }
}
