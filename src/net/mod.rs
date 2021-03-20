use std::{
    fmt::Debug,
    hash::Hash,
    io::{self, Write},
};

mod storage;
use storage::Storage;

mod vis;

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

impl<T: Storage + Clone> Net<T> {
    pub fn render_dot<W: Write>(&self, output: &mut W) -> io::Result<()>
    where
        T: Copy + Eq + Hash,
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

    pub fn reduce(&mut self) {
        while let Some((a, b)) = self.active.pop() {
            self.rewrite(a, b);
        }
    }

    fn rewrite(&mut self, x: Index, y: Index) {
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
