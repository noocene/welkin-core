use super::Index;

impl Index {
    pub fn within(&self, other: Index) -> bool {
        self.0 > other.0
    }
    pub fn above(&self, other: Index) -> bool {
        self.0 < other.0
    }
    pub fn parent(&self) -> Index {
        let mut parent = *self;
        parent.0 -= 1;
        parent
    }
    pub fn child(&self) -> Index {
        let mut child = *self;
        child.0 += 1;
        child
    }
    pub fn top() -> Self {
        Index(0)
    }
}
