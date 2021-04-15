use derivative::Derivative;

use crate::{
    net::{AgentExt, AgentType, NetBuilder, PortExt, Slot, VisitNet},
    term::{Definitions, Index, Show, Stratified, Term},
};

#[derive(Derivative)]
#[derivative(Debug(bound = "T: Show"))]
pub enum NetError<T> {
    TypedTerm(Term<T>),
}

impl<T> Term<T> {
    fn build_net<U: Definitions<T>, N: NetBuilder>(
        &self,
        net: &mut N,
        definitions: &U,
        var_ptrs: &mut Vec<N::Port>,
    ) -> Result<N::Port, NetError<T>>
    where
        T: Clone,
        N::Port: PartialEq + Clone,
    {
        use Term::*;

        Ok(match self {
            Variable(symbol) => {
                let ptr = var_ptrs.iter().rev().nth(symbol.0).unwrap().clone();
                let target = net.follow(ptr.clone());
                if target.is_root() || target == ptr {
                    ptr
                } else {
                    let (principal, left, right) = net.add(AgentType::Zeta);
                    net.connect(principal, ptr);
                    net.connect(left, target);
                    right
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
                    let (principal, left, right) = net.add(AgentType::Delta);
                    var_ptrs.push(left.clone());
                    let body = body.build_net(net, definitions, var_ptrs)?;
                    var_ptrs.pop();
                    net.connect(right, body);
                    principal
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
                    let (principal, left, right) = net.add(AgentType::Delta);
                    let function = function.build_net(net, definitions, var_ptrs)?;
                    net.connect(principal, function);
                    let argument = argument.build_net(net, definitions, var_ptrs)?;
                    net.connect(left, argument);
                    right
                }
            }
            Annotation { expression, .. } => expression.build_net(net, definitions, var_ptrs)?,
            _ => Err(NetError::TypedTerm(self.clone()))?,
        })
    }
}

mod sealed {
    use crate::net::NetBuilder;

    pub trait Sealed {}

    impl<T: NetBuilder> Sealed for T {}
}

pub trait NetBuilderExt<T, U: Definitions<T>>: NetBuilder + sealed::Sealed {
    fn build_net(terms: Stratified<'_, T, U>) -> Result<Self::Net, NetError<T>>
    where
        Self: Sized;
}

impl<T: NetBuilder, V: Clone, U: Definitions<V>> NetBuilderExt<V, U> for T
where
    T::Port: PartialEq + Clone,
{
    fn build_net(terms: Stratified<'_, V, U>) -> Result<T::Net, NetError<V>>
    where
        Self: Sized,
    {
        let mut net = T::new();
        let mut var_ptrs = vec![];
        let entry = terms.0.build_net(&mut net, terms.1, &mut var_ptrs)?;
        Ok(net.build(entry))
    }
}

fn build_term<T, N: VisitNet>(
    net: &N,
    port: N::Port,
    var_ptrs: &mut Vec<N::Port>,
    dup_exit: &mut Vec<Slot>,
) -> Term<T>
where
    N::Port: PartialEq,
{
    use Slot::*;

    let agent = net.get(port.address());
    let ty = agent.ty();

    if ty == AgentType::Delta {
        match port.slot() {
            Principal => {
                var_ptrs.push(<N::Port as PortExt>::new(port.address(), Slot::Left));

                let b_port = net.follow(<N::Port as PortExt>::new(port.address(), Slot::Right));
                let body = Box::new(build_term(net, b_port, var_ptrs, dup_exit));

                var_ptrs.pop();

                Term::Lambda {
                    body,
                    erased: false,
                }
            }
            Left => Term::Variable(Index(
                var_ptrs
                    .iter()
                    .rev()
                    .enumerate()
                    .find(|a| a.1 == &port)
                    .unwrap()
                    .0,
            )),
            Right => {
                let a_port = net.follow(<N::Port as PortExt>::new(port.address(), Slot::Left));
                let argument = Box::new(build_term(net, a_port, var_ptrs, dup_exit));

                let a_port = net.follow(<N::Port as PortExt>::new(port.address(), Slot::Principal));
                let function = Box::new(build_term(net, a_port, var_ptrs, dup_exit));

                Term::Apply {
                    function,
                    argument,
                    erased: false,
                }
            }
        }
    } else {
        match port.slot() {
            Slot::Principal => {
                let exit = dup_exit.pop().unwrap();
                let term = build_term(
                    net,
                    net.follow(<N::Port as PortExt>::new(port.address(), exit)),
                    var_ptrs,
                    dup_exit,
                );
                dup_exit.push(exit);
                term
            }
            _ => {
                dup_exit.push(port.slot());
                let term = build_term(
                    net,
                    net.follow(<N::Port as PortExt>::new(port.address(), Slot::Principal)),
                    var_ptrs,
                    dup_exit,
                );
                dup_exit.pop();
                term
            }
        }
    }
}

pub trait VisitNetExt: VisitNet
where
    Self::Port: PartialEq,
{
    fn read_term<T>(&self, port: Self::Port) -> Term<T>;
}

impl<T: VisitNet> VisitNetExt for T
where
    Self::Port: PartialEq,
{
    fn read_term<U>(&self, port: <Self as VisitNet>::Port) -> Term<U> {
        build_term(self, port, &mut vec![], &mut vec![])
    }
}
