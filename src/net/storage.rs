use super::Slot;

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
