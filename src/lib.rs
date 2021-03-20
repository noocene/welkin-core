use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Debug,
    hash::Hash,
    io::{self, Write},
};

// pub mod term;

mod sealed {
    pub trait Sealed {}
}

pub trait Storage: sealed::Sealed {
    const MAX_NODES: usize;

    fn pack(index: usize, slot: Slot) -> Self;
    fn slot(&self) -> Slot;
    fn address(&self) -> usize;
}

macro_rules! impl_storage {
    ($($t:ty),+) => {
        $(
            impl sealed::Sealed for $t {}

            impl Storage for $t {
                const MAX_NODES: usize = ((<$t>::MAX >> 2) + 1) as usize;

                fn pack(index: usize, slot: Slot) -> Self {
                    use Slot::*;

                    let slot: $t = match slot {
                        Principal => 0,
                        Left => 1,
                        Right => 2,
                    };

                    index as $t << 2 | slot
                }

                fn slot(&self) -> Slot {
                    use Slot::*;

                    match *self & 3 {
                        0 => Principal,
                        1 => Left,
                        2 => Right,
                        _ => panic!("invalid slot")
                    }
                }

                fn address(&self) -> usize {
                    (*self >> 2) as usize
                }
            }
        )+
    };
}

impl_storage!(u8, u16, u32, u64, u128);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Epsilon,
    Delta,
    Zeta,
    Root,
}

#[derive(Debug, Clone)]
pub struct Agent<T: Storage> {
    left: Port<T>,
    right: Port<T>,
    principal: Port<T>,
    ty: AgentType,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Left,
    Right,
    Principal,
}

pub struct Ports<T: Storage> {
    pub left: Port<T>,
    pub right: Port<T>,
    pub principal: Port<T>,
}

impl<T: Storage + Clone> Ports<T> {
    fn new(agent: &Agent<T>) -> Self {
        Ports {
            left: agent.left(),
            right: agent.right(),
            principal: agent.principal(),
        }
    }
}

impl<T: Storage + Clone> Agent<T> {
    fn new(principal: Port<T>, left: Port<T>, right: Port<T>, ty: AgentType) -> Self {
        Agent {
            left,
            right,
            principal,
            ty,
        }
    }

    pub fn left(&self) -> Port<T> {
        self.left.clone()
    }

    pub fn right(&self) -> Port<T> {
        self.right.clone()
    }

    pub fn principal(&self) -> Port<T> {
        self.principal.clone()
    }

    pub fn ports(&self) -> Ports<T> {
        Ports::new(self)
    }

    pub fn slot(&self, slot: Slot) -> Port<T> {
        use Slot::*;

        match slot {
            Left => self.left(),
            Right => self.right(),
            Principal => self.principal(),
        }
    }

    pub fn update_slot(&mut self, slot: Slot, port: Port<T>) {
        use Slot::*;

        *match slot {
            Left => &mut self.left,
            Right => &mut self.right,
            Principal => &mut self.principal,
        } = port;
    }

    pub fn ty(&self) -> AgentType {
        self.ty
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Port<T: Storage>(T);

impl<T: Storage + Debug> Debug for Port<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Port({:?}, {:?})", self.address(), self.slot())
    }
}

impl<T: Storage> Port<T> {
    fn new(node: Index, slot: Slot) -> Self {
        Port(T::pack(node.0, slot))
    }

    fn address(&self) -> Index {
        Index(self.0.address())
    }

    fn slot(&self) -> Slot {
        self.0.slot()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Index(usize);

#[derive(Debug)]
pub struct Net<T: Storage> {
    agents: Vec<Agent<T>>,
    freed: Vec<Index>,
    active: Vec<(Index, Index)>,
}

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
            },
            n.0 .0
        )))
    }

    fn kind(&self) -> dot::Kind {
        dot::Kind::Graph
    }
}

impl<'a, T: Eq + Debug + Hash + Storage + Clone + Copy>
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

impl<T: Storage + Clone> Net<T> {
    pub fn render_dot<W: Write>(&self, output: &mut W) -> io::Result<()>
    where
        T: Copy + Eq + Debug + Hash,
    {
        dot::render(self, output)
    }

    pub fn new() -> (Self, Port<T>) {
        let mut net = Net {
            agents: vec![],
            freed: vec![],
            active: vec![],
        };
        let root = net.add(AgentType::Root);
        let p = root.principal();
        (net, p)
    }

    pub fn add(&mut self, ty: AgentType) -> &Agent<T> {
        let (idx, extant) = self
            .freed
            .pop()
            .map(|idx| (idx, true))
            .unwrap_or_else(|| (Index(self.agents.len()), false));

        let (principal, left, right) = (
            Port::new(idx, Slot::Principal),
            Port::new(idx, Slot::Left),
            Port::new(idx, Slot::Right),
        );

        let agent = Agent::new(principal, left, right, ty);

        if extant {
            self.agents[idx.0] = agent;
        } else {
            self.agents.push(agent);
        }

        self.get(idx)
    }

    pub fn get(&self, index: Index) -> &Agent<T> {
        &self.agents[index.0]
    }

    pub fn get_mut(&mut self, index: Index) -> &mut Agent<T> {
        &mut self.agents[index.0]
    }

    pub fn follow(&self, port: Port<T>) -> Port<T> {
        self.get_agent(&port).slot(port.slot())
    }

    pub fn connect(&mut self, a: Port<T>, b: Port<T>) {
        use Slot::Principal;

        if a.slot() == Principal && b.slot() == Principal {
            self.active.push((a.address(), b.address()));
        }

        let a_agent = self.get_mut(a.address());
        let b_addr = b.address();
        a_agent.update_slot(a.slot(), b.clone());
        let b_agent = self.get_mut(b_addr);
        b_agent.update_slot(b.slot(), a);
    }

    fn free(&mut self, address: Index) {
        self.freed.push(address);
    }

    fn get_agent(&self, x: &Port<T>) -> &Agent<T> {
        self.get(x.address())
    }

    pub fn reduce(&mut self)
    where
        T: Debug,
    {
        while let Some((a, b)) = self.active.pop() {
            self.rewrite(a, b);
        }
    }

    fn rewrite(&mut self, x: Index, y: Index)
    where
        T: Debug,
    {
        use AgentType::{Epsilon, Root};

        let (x_ty, x_ports) = {
            let x = self.get(x);
            (x.ty(), x.ports())
        };
        let (y_ty, y_ports) = {
            let y = self.get(y);
            (y.ty(), y.ports())
        };

        if x_ty == Root || y_ty == Root {
            return;
        }

        if x_ty == y_ty {
            if x_ty != Epsilon {
                self.connect(x_ports.right, y_ports.right);
                self.connect(x_ports.left, y_ports.left);
            }

            self.free(x);
            self.free(y);
        } else {
            if x_ty == Epsilon || y_ty == Epsilon {
                let (e, ne, ports) = if x_ty == Epsilon {
                    (x, y, y_ports)
                } else {
                    (y, x, x_ports)
                };
                let era = self.add(Epsilon).ports();
                self.connect(era.left, era.right);
                self.connect(era.principal, ports.left);
                self.connect(Port::new(e, Slot::Principal), ports.right);
                self.free(ne);
                return;
            }

            let dup_x = self.add(x_ty).ports();
            let dup_y = self.add(y_ty).ports();

            self.connect(dup_x.principal, y_ports.left);
            self.connect(x_ports.principal, y_ports.right);
            self.connect(dup_y.principal, x_ports.left);
            self.connect(y_ports.principal, x_ports.right);

            self.connect(dup_x.left, dup_y.right);
            self.connect(dup_x.right, Port::new(y, Slot::Right));

            self.connect(Port::new(x, Slot::Left), dup_y.left);
            self.connect(Port::new(x, Slot::Right), Port::new(y, Slot::Left));
        }
    }
}
