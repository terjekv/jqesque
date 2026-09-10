use std::fmt;

use serde::{
    de::{DeserializeSeed, MapAccess, SeqAccess, Visitor},
    forward_to_deserialize_any, Deserialize, Deserializer,
};
use serde_json::{Number, Value};

use super::{
    check_limit, JqesqueError, LimitKind, DEFAULT_MAX_ARRAY_SLOTS, DEFAULT_MAX_INPUT_BYTES,
    DEFAULT_MAX_VALUE_DEPTH, DEFAULT_MAX_VALUE_NODES,
};

const _: () = assert!(DEFAULT_MAX_VALUE_NODES <= DEFAULT_MAX_ARRAY_SLOTS);

#[derive(Debug, Clone)]
pub(crate) struct ValidatedValue {
    pub(super) value: Value,
    pub(super) array_slots: usize,
    pub(super) nodes: usize,
    pub(super) bytes: usize,
}
impl PartialEq for ValidatedValue {
    fn eq(&self, other: &Self) -> bool {
        crate::manipulators::numeric_json_eq(&self.value, &other.value)
    }
}

impl ValidatedValue {
    pub(crate) fn new(value: Value) -> Result<Self, JqesqueError> {
        match inspect(&value) {
            Ok(budget) => Ok(Self {
                value,
                array_slots: budget.slots,
                nodes: budget.nodes,
                bytes: budget.bytes,
            }),
            Err(error) => {
                discard(value);
                Err(error)
            }
        }
    }
}

#[derive(Default)]
struct Budget {
    nodes: usize,
    slots: usize,
    bytes: usize,
}
impl Budget {
    fn node(&mut self, depth: usize, array_slot: bool) -> Result<(), JqesqueError> {
        check_limit(LimitKind::ValueDepth, depth, DEFAULT_MAX_VALUE_DEPTH)?;
        self.nodes = self.nodes.saturating_add(1);
        check_limit(LimitKind::ValueNodes, self.nodes, DEFAULT_MAX_VALUE_NODES)?;
        if array_slot {
            // Every array slot is also a counted node. The node ceiling is stricter
            // than the array-slot ceiling, so the checked node count proves this bound.
            self.slots += 1;
        }
        Ok(())
    }
    fn bytes(&mut self, amount: usize) -> Result<(), JqesqueError> {
        self.bytes = self.bytes.saturating_add(amount);
        check_limit(LimitKind::ValueBytes, self.bytes, DEFAULT_MAX_INPUT_BYTES)
    }
}

fn unsigned_bytes(value: u64) -> usize {
    if value < 10 {
        1
    } else if value < 100 {
        2
    } else {
        value.ilog10() as usize + 1
    }
}
fn signed_bytes(value: i64) -> usize {
    unsigned_bytes(value.unsigned_abs()) + usize::from(value < 0)
}
fn number_bytes(number: &Number) -> usize {
    if let Some(value) = number.as_u64() {
        unsigned_bytes(value)
    } else if let Some(value) = number.as_i64() {
        signed_bytes(value)
    } else {
        number.to_string().len()
    }
}

fn inspect(value: &Value) -> Result<Budget, JqesqueError> {
    let mut budget = Budget::default();
    if !value.is_array() && !value.is_object() {
        budget.node(0, false)?;
        match value {
            Value::String(value) => budget.bytes(value.len())?,
            Value::Number(number) => budget.bytes(number_bytes(number))?,
            _ => {}
        }
        return Ok(budget);
    }
    let mut pending = vec![(value, 0, false)];
    while let Some((value, depth, slot)) = pending.pop() {
        budget.node(depth, slot)?;
        match value {
            Value::Array(values) => {
                check_limit(
                    LimitKind::ValueNodes,
                    budget
                        .nodes
                        .saturating_add(pending.len())
                        .saturating_add(values.len()),
                    DEFAULT_MAX_VALUE_NODES,
                )?;
                pending.extend(values.iter().map(|v| (v, depth + 1, true)));
            }
            Value::Object(values) => {
                check_limit(
                    LimitKind::ValueNodes,
                    budget
                        .nodes
                        .saturating_add(pending.len())
                        .saturating_add(values.len()),
                    DEFAULT_MAX_VALUE_NODES,
                )?;
                for (key, value) in values {
                    budget.bytes(key.len())?;
                    pending.push((value, depth + 1, false));
                }
            }
            Value::String(value) => budget.bytes(value.len())?,
            Value::Number(number) => budget.bytes(number_bytes(number))?,
            _ => {}
        }
    }
    Ok(budget)
}

/// Safe rejection of caller-constructed deeply nested values without recursive Drop.
pub(crate) fn discard(value: Value) {
    let mut pending = match value {
        Value::Array(values) => values,
        Value::Object(values) => values.into_iter().map(|(_, value)| value).collect(),
        _ => return,
    };
    while let Some(value) = pending.pop() {
        match value {
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => pending.extend(values.into_iter().map(|(_, value)| value)),
            _ => {}
        }
    }
}

pub(super) fn check_clone(value: &Value) -> Result<(), JqesqueError> {
    // A document produced by a bounded path plus a bounded payload can reach depth 256.
    let mut pending = vec![(value, 0)];
    while let Some((value, depth)) = pending.pop() {
        check_limit(
            LimitKind::DocumentDepth,
            depth,
            super::DEFAULT_MAX_PATH_DEPTH + DEFAULT_MAX_VALUE_DEPTH,
        )?;
        match value {
            Value::Array(values) => pending.extend(values.iter().map(|value| (value, depth + 1))),
            Value::Object(values) => {
                pending.extend(values.values().map(|value| (value, depth + 1)))
            }
            _ => {}
        }
    }
    Ok(())
}

impl<'de> Deserialize<'de> for ValidatedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut budget = Budget::default();
        let value = Value::deserialize(Limited {
            inner: deserializer,
            budget: &mut budget,
            depth: 0,
            node: true,
            slot: false,
        })?;
        Ok(Self {
            value,
            array_slots: budget.slots,
            nodes: budget.nodes,
            bytes: budget.bytes,
        })
    }
}

// Wrap Serde's own Value decoder instead of duplicating its number/map representation rules.
// Sequence hints are hidden so a hostile deserializer cannot force a speculative allocation.
struct Limited<'a, D> {
    inner: D,
    budget: &'a mut Budget,
    depth: usize,
    node: bool,
    slot: bool,
}
impl<'de, D: Deserializer<'de>> Deserializer<'de> for Limited<'_, D> {
    type Error = D::Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        if self.node {
            self.budget
                .node(self.depth, self.slot)
                .map_err(serde::de::Error::custom)?;
        }
        self.inner.deserialize_any(LimitedVisitor {
            inner: visitor,
            budget: self.budget,
            depth: self.depth,
        })
    }
    forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct LimitedVisitor<'a, V> {
    inner: V,
    budget: &'a mut Budget,
    depth: usize,
}
impl<'de, V: Visitor<'de>> Visitor<'de> for LimitedVisitor<'_, V> {
    type Value = V::Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(f)
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
        self.inner.visit_bool(value)
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        self.budget.bytes(signed_bytes(value)).map_err(E::custom)?;
        self.inner.visit_i64(value)
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        self.budget
            .bytes(unsigned_bytes(value))
            .map_err(E::custom)?;
        self.inner.visit_u64(value)
    }
    fn visit_i128<E: serde::de::Error>(self, value: i128) -> Result<Self::Value, E> {
        self.budget
            .bytes(
                value.unsigned_abs().checked_ilog10().unwrap_or(0) as usize
                    + 1
                    + usize::from(value < 0),
            )
            .map_err(E::custom)?;
        self.inner.visit_i128(value)
    }
    fn visit_u128<E: serde::de::Error>(self, value: u128) -> Result<Self::Value, E> {
        self.budget
            .bytes(value.checked_ilog10().unwrap_or(0) as usize + 1)
            .map_err(E::custom)?;
        self.inner.visit_u128(value)
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        self.budget
            .bytes(Number::from_f64(value).map_or(0, |number| number.to_string().len()))
            .map_err(E::custom)?;
        self.inner.visit_f64(value)
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_unit()
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_none()
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.budget.bytes(value.len()).map_err(E::custom)?;
        self.inner.visit_str(value)
    }
    fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.budget.bytes(value.len()).map_err(E::custom)?;
        self.inner.visit_borrowed_str(value)
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.budget.bytes(value.len()).map_err(E::custom)?;
        self.inner.visit_string(value)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_seq(LimitedSeq {
            inner: access,
            budget: self.budget,
            depth: self.depth + 1,
        })
    }
    fn visit_map<A: MapAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_map(LimitedMap {
            inner: access,
            budget: self.budget,
            depth: self.depth + 1,
        })
    }
}

struct LimitedSeed<'a, S> {
    inner: S,
    budget: &'a mut Budget,
    depth: usize,
    node: bool,
    slot: bool,
}
impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for LimitedSeed<'_, S> {
    type Value = S::Value;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self.inner.deserialize(Limited {
            inner: deserializer,
            budget: self.budget,
            depth: self.depth,
            node: self.node,
            slot: self.slot,
        })
    }
}
struct LimitedSeq<'a, A> {
    inner: A,
    budget: &'a mut Budget,
    depth: usize,
}
impl<'de, A: SeqAccess<'de>> SeqAccess<'de> for LimitedSeq<'_, A> {
    type Error = A::Error;
    fn next_element_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, A::Error> {
        self.inner.next_element_seed(LimitedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth,
            node: true,
            slot: true,
        })
    }
}
struct LimitedMap<'a, A> {
    inner: A,
    budget: &'a mut Budget,
    depth: usize,
}
impl<'de, A: MapAccess<'de>> MapAccess<'de> for LimitedMap<'_, A> {
    type Error = A::Error;
    fn next_key_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, A::Error> {
        self.inner.next_key_seed(LimitedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth,
            node: false,
            slot: false,
        })
    }
    fn next_value_seed<S: DeserializeSeed<'de>>(&mut self, seed: S) -> Result<S::Value, A::Error> {
        self.inner.next_value_seed(LimitedSeed {
            inner: seed,
            budget: self.budget,
            depth: self.depth,
            node: true,
            slot: false,
        })
    }
}
