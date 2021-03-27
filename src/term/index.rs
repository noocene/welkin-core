use super::Index;

impl Index {
    pub(crate) fn within(&self, other: Index) -> bool {
        self.0 > other.0
    }
    pub(crate) fn parent(&self) -> Index {
        let mut parent = *self;
        parent.0 -= 1;
        parent
    }
    pub(crate) fn child(&self) -> Index {
        let mut child = *self;
        child.0 += 1;
        child
    }
    pub(crate) fn top() -> Self {
        Index(0)
    }
    pub fn value(self) -> usize {
        self.0
    }
}
