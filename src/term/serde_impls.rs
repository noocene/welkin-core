use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    alloc::{Allocator, Zero},
    Primitives, Term,
};

pub(crate) fn serialize<
    T: Serialize,
    U: Primitives<T> + Serialize,
    A: Allocator<T, U>,
    S: Serializer,
>(
    data: &A::Box,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    (&**data).serialize(serializer)
}

pub(crate) fn deserialize<
    'de,
    T: Deserialize<'de>,
    U: Primitives<T> + Deserialize<'de>,
    A: Allocator<T, U> + Zero,
    D: Deserializer<'de>,
>(
    deserializer: D,
) -> Result<A::Box, D::Error> {
    Term::<T, U, A>::deserialize(deserializer).map(|term| {
        let alloc = A::zero();
        alloc.alloc(term)
    })
}
