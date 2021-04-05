use std::{borrow::Cow, collections::HashSet, hash::Hash};

use super::{Agent, AgentType, Index, Net, Port, Slot, Storage};

impl<'a, T: Storage + Clone + Copy> dot::Labeller<'a, (Index, Agent<T>), (Port<T>, Port<T>)>
    for Net<T>
{
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("example1").unwrap()
    }

    fn node_id(&'a self, n: &(Index, Agent<T>)) -> dot::Id<'a> {
        dot::Id::new(format!("A{}", n.0 .0)).unwrap()
    }

    fn node_shape(&self, n: &(Index, Agent<T>)) -> Option<dot::LabelText<'a>> {
        Some(dot::LabelText::LabelStr(Cow::Owned(format!(
            "{}",
            match n.1.ty() {
                AgentType::Root => "diamond",
                AgentType::Epsilon => "circle",
                _ => "square",
            }
        ))))
    }

    fn edge_ports(&self, e: &(Port<T>, Port<T>)) -> Option<(dot::Port, dot::Port)> {
        use Slot::*;
        let (a, b) = e;
        let ap;
        let bp;
        let aty = self.get(a.address()).ty();
        if aty != AgentType::Epsilon && aty != AgentType::Root {
            ap = match a.slot() {
                Left => dot::Port::W,
                Right => dot::Port::E,
                Principal => dot::Port::S,
            }
        } else {
            ap = dot::Port::S;
        }
        let bty = self.get(b.address()).ty();
        if bty != AgentType::Epsilon && bty != AgentType::Root {
            bp = match b.slot() {
                Left => dot::Port::W,
                Right => dot::Port::E,
                Principal => dot::Port::N,
            }
        } else {
            bp = dot::Port::N;
        }
        Some((bp, ap))
    }

    fn edge_color(&self, e: &(Port<T>, Port<T>)) -> Option<dot::LabelText<'a>> {
        use Slot::Principal;

        Some(dot::LabelText::LabelStr(Cow::Owned(format!(
            "{}",
            if e.0.slot() == Principal && e.1.slot() == Principal {
                "red"
            } else {
                "black"
            }
        ))))
    }

    fn node_label<'b>(&'b self, n: &(Index, Agent<T>)) -> dot::LabelText<'b> {
        use AgentType::*;

        dot::LabelText::LabelStr(Cow::Owned(format!(
            "{}{}",
            match n.1.ty() {
                Epsilon => "&epsilon;",
                Root => "*",
                Delta => "&delta;",
                Zeta => "&zeta;",

                _ => "WIRE",
            },
            n.0 .0
        )))
    }

    fn kind(&self) -> dot::Kind {
        dot::Kind::Graph
    }
}

impl<'a, T: Eq + Hash + Storage + Clone + Copy>
    dot::GraphWalk<'a, (Index, Agent<T>), (Port<T>, Port<T>)> for Net<T>
{
    fn nodes(&self) -> dot::Nodes<'a, (Index, Agent<T>)> {
        Cow::Owned(
            self.agents
                .clone()
                .into_iter()
                .enumerate()
                .filter_map(|(idx, agent)| {
                    if !self.freed.contains(&Index(idx)) {
                        Some((Index(idx), agent))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        )
    }

    fn edges(&'a self) -> dot::Edges<'a, (Port<T>, Port<T>)> {
        let edges = self
            .agents
            .clone()
            .into_iter()
            .map(|agent| {
                let ports = agent.ports();
                vec![ports.principal, ports.left, ports.right]
                    .into_iter()
                    .map(move |port| {
                        let mut out = (self.follow(port), port);
                        if out.0.address() < out.1.address() {
                            out = (out.1, out.0);
                        }
                        out
                    })
            })
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|(a, b)| {
                // don't render intended self-referential ports
                if a.address() == b.address() && {
                    let ty = self.get(a.address()).ty();
                    ty == AgentType::Epsilon || ty == AgentType::Root
                } {
                    return false;
                }
                !(self.freed.contains(&a.address()) || self.freed.contains(&b.address()))
            })
            .collect::<Vec<_>>();
        Cow::Owned(edges)
    }

    fn source(&'a self, e: &(Port<T>, Port<T>)) -> (Index, Agent<T>) {
        (e.0.address(), self.get(e.0.address()).clone())
    }

    fn target(&'a self, e: &(Port<T>, Port<T>)) -> (Index, Agent<T>) {
        (e.1.address(), self.get(e.1.address()).clone())
    }
}
