use std::{fmt, marker::PhantomData};

use serde::{
    de::{DeserializeSeed, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use super::{check_limit, JqesqueError, LimitKind};

/// Read a sequence incrementally, checking its length before decoding an excess element.
/// The callback can enforce cumulative budgets before retaining each element.
pub(super) fn bounded_sequence<'de, D, T, F>(
    deserializer: D,
    limit: usize,
    kind: LimitKind,
    check: F,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
    F: FnMut(&T) -> Result<(), JqesqueError>,
{
    struct Sequence<T, F> {
        limit: usize,
        kind: LimitKind,
        check: F,
        marker: PhantomData<T>,
    }
    impl<'de, T: Deserialize<'de>, F: FnMut(&T) -> Result<(), JqesqueError>> Visitor<'de>
        for Sequence<T, F>
    {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a bounded sequence")
        }
        fn visit_seq<A: SeqAccess<'de>>(mut self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = access.next_element_seed(Element {
                count: values.len() + 1,
                limit: self.limit,
                kind: self.kind,
                marker: PhantomData,
            })? {
                (self.check)(&value).map_err(serde::de::Error::custom)?;
                values.push(value);
            }
            Ok(values)
        }
    }
    struct Element<T> {
        count: usize,
        limit: usize,
        kind: LimitKind,
        marker: PhantomData<T>,
    }
    impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for Element<T> {
        type Value = T;
        fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
            check_limit(self.kind, self.count, self.limit).map_err(serde::de::Error::custom)?;
            T::deserialize(deserializer)
        }
    }
    deserializer.deserialize_seq(Sequence {
        limit,
        kind,
        check,
        marker: PhantomData,
    })
}
