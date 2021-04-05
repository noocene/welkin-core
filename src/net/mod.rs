use std::{fmt::Debug, hash::Hash};

mod storage;
pub(crate) use storage::Storage;

#[cfg(feature = "graphviz")]
mod vis;

pub trait PortExt {
    fn is_root(&self) -> bool;
}

pub trait NetBuilder {
    type Net;
    type Port: PortExt;

    fn new() -> Self;

    fn add(&mut self, ty: AgentType) -> (Self::Port, Self::Port, Self::Port);
    fn connect(&mut self, a: Self::Port, b: Self::Port);

    fn follow(&self, from: Self::Port) -> Self::Port;

    fn build(self, root: Self::Port) -> Self::Net;
}

impl<T: Storage + Clone + Eq> NetBuilder for Net<T> {
    type Net = Self;
    type Port = Port<T>;

    fn new() -> Self {
        Net::new().0
    }

    fn add(&mut self, ty: AgentType) -> (Self::Port, Self::Port, Self::Port) {
        let ports = Net::<T>::add(self, ty).ports();
        (ports.principal, ports.left, ports.right)
    }

    fn connect(&mut self, a: Self::Port, b: Self::Port) {
        Net::<T>::connect(self, a, b)
    }

    fn follow(&self, from: Self::Port) -> Self::Port {
        Net::<T>::follow(self, from)
    }

    fn build(mut self, root: Self::Port) -> Self::Net {
        self.connect(self.get(Index(0)).ports().right, root);
        self.bind_unbound();
        self
    }
}

impl<T: Storage> PortExt for Port<T> {
    fn is_root(&self) -> bool {
        self.address().is_root()
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Epsilon,
    Delta,
    Zeta,
    Root,
}

#[derive(Debug, Clone)]
#[repr(C)]
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

    pub(crate) fn address(&self) -> Index {
        Index(self.0.address())
    }

    fn slot(&self) -> Slot {
        self.0.slot()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Index(usize);

impl Index {
    pub(crate) fn is_root(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug)]
pub struct Net<T: Storage> {
    agents: Vec<Agent<T>>,
    freed: Vec<Index>,
    active: Vec<(Index, Index)>,
}

impl<T: Storage + Clone> Net<T> {
    #[cfg(feature = "graphviz")]
    pub fn render_to<W: std::io::Write>(&self, output: &mut W) -> std::io::Result<()>
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
        let root = net.add(AgentType::Root).ports();
        net.connect(root.principal, root.left);
        let p = root.right;
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

    pub fn disconnect(&mut self, a: Port<T>)
    where
        T: Eq,
    {
        let b = self.follow(a.clone());
        if self.follow(b.clone()) == a {
            let aa = a.address();
            let ba = b.address();

            self.get_mut(aa).update_slot(a.slot(), a);
            self.get_mut(ba).update_slot(b.slot(), b);
        }
    }

    fn free(&mut self, address: Index) {
        self.freed.push(address);
    }

    fn get_agent(&self, x: &Port<T>) -> &Agent<T> {
        self.get(x.address())
    }

    pub fn reduce(&mut self, max_rewrites: Option<usize>) -> usize {
        let mut rewrites = 0;

        while let Some((a, b)) = self.active.pop() {
            self.rewrite(a, b);
            rewrites += 1;
            match max_rewrites {
                Some(max) if max == rewrites => break,
                _ => {}
            }
        }

        rewrites
    }

    pub fn reduce_all(&mut self) -> usize {
        self.reduce(None)
    }

    pub(crate) fn bind_unbound(&mut self)
    where
        T: Eq,
    {
        for i in 0..self.agents.len() {
            let ports = self.agents[i].ports();
            if self.follow(ports.left.clone()) == ports.left {
                let era = self.add(AgentType::Epsilon).ports();
                self.connect(ports.left, era.principal);
            }
        }
    }

    fn rewrite(&mut self, x: Index, y: Index) {
        use AgentType::Epsilon;

        let x_ty = self.get(x).ty();
        let y_ty = self.get(y).ty();

        if x_ty == y_ty {
            if x_ty != Epsilon {
                let p0 = self.follow(Port::new(x, Slot::Left));
                let p1 = self.follow(Port::new(y, Slot::Left));
                self.connect(p0, p1);
                let p0 = self.follow(Port::new(x, Slot::Right));
                let p1 = self.follow(Port::new(y, Slot::Right));
                self.connect(p0, p1);
            }

            self.free(x);
            self.free(y);
        } else {
            use Slot::*;

            if x_ty == Epsilon || y_ty == Epsilon {
                let (x, y) = if x_ty == Epsilon { (x, y) } else { (y, x) };
                let p = self.add(Epsilon).ports().principal.address();
                let q = self.add(Epsilon).ports().principal.address();

                self.connect(Port::new(p, Principal), self.follow(Port::new(y, Left)));
                self.connect(Port::new(q, Principal), self.follow(Port::new(y, Right)));

                self.free(x);
                self.free(y);

                return;
            }

            let p = self.add(y_ty).ports().principal.address();
            let q = self.add(y_ty).ports().principal.address();
            let r = self.add(x_ty).ports().principal.address();
            let s = self.add(x_ty).ports().principal.address();

            self.connect(Port::new(r, Left), Port::new(p, Left));
            self.connect(Port::new(s, Left), Port::new(p, Right));
            self.connect(Port::new(r, Right), Port::new(q, Left));
            self.connect(Port::new(s, Right), Port::new(q, Right));

            self.connect(Port::new(p, Principal), self.follow(Port::new(x, Left)));
            self.connect(Port::new(q, Principal), self.follow(Port::new(x, Right)));
            self.connect(Port::new(r, Principal), self.follow(Port::new(y, Left)));
            self.connect(Port::new(s, Principal), self.follow(Port::new(y, Right)));

            self.free(x);
            self.free(y);
        }
    }
}
