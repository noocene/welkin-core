use super::Slot;

mod sealed {
    pub trait Sealed {}
}

pub trait Storage: sealed::Sealed {
    const MAX_NODES: usize;

    fn pack(index: Self, slot: Slot) -> Self;
    fn slot(&self) -> Slot;
    fn address(&self) -> Self;
    fn zero() -> Self;
    fn from_usize(data: usize) -> Self;
    fn into_usize(self) -> usize;
}

macro_rules! impl_storage {
    ($($t:ty),+) => {
        $(
            impl sealed::Sealed for $t {}

            impl Storage for $t {
                const MAX_NODES: usize = ((<$t>::MAX >> 2) + 1) as usize;

                fn pack(index: Self, slot: Slot) -> Self {
                    use Slot::*;

                    let slot: $t = match slot {
                        Principal => 0,
                        Left => 1,
                        Right => 2,
                    };

                    index as $t << 2 | slot
                }

                fn zero() -> Self {
                    0
                }

                fn slot(&self) -> Slot {
                    use Slot::*;

                    match *self & 3 {
                        0 => Principal,
                        1 => Left,
                        2 => Right,
                        invalid => panic!("invalid slot {}", invalid)
                    }
                }

                fn address(&self) -> Self {
                    (*self >> 2) as Self
                }

                fn from_usize(data: usize) -> Self {
                    data as Self
                }

                fn into_usize(self) -> usize {
                    self as usize
                }
            }
        )+
    };
}

impl_storage!(u8, u16, u32, u64, u128);
