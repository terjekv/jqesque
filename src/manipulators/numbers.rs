use serde::{
    ser::{Error as _, Impossible, SerializeStruct},
    Serialize, Serializer,
};
use serde_json::{Error, Number};

// Number serializes directly as a primitive by default, and through a struct when
// serde_json arbitrary_precision is active. Detect through Serde so feature unification
// by downstream crates is respected, even without jqesque's feature being selected.
struct NumberEncoding;
impl SerializeStruct for NumberEncoding {
    type Ok = bool;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        _: &T,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
macro_rules! primitive {
    ($($name:ident($ty:ty)),* $(,)?) => { $(
        fn $name(self, _: $ty) -> Result<bool, Self::Error> { Ok(false) }
    )* };
}
impl Serializer for NumberEncoding {
    type Ok = bool;
    type Error = Error;
    type SerializeSeq = Impossible<bool, Self::Error>;
    type SerializeTuple = Impossible<bool, Self::Error>;
    type SerializeTupleStruct = Impossible<bool, Self::Error>;
    type SerializeTupleVariant = Impossible<bool, Self::Error>;
    type SerializeMap = Impossible<bool, Self::Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<bool, Self::Error>;
    primitive! { serialize_bool(bool), serialize_i8(i8), serialize_i16(i16), serialize_i32(i32), serialize_i64(i64), serialize_i128(i128), serialize_u8(u8), serialize_u16(u16), serialize_u32(u32), serialize_u64(u64), serialize_u128(u128), serialize_f32(f32), serialize_f64(f64), serialize_char(char), serialize_str(&str), serialize_bytes(&[u8]) }
    fn serialize_none(self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<bool, Self::Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<bool, Self::Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result<bool, Self::Error> {
        value.serialize(self)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(Error::custom("unexpected number sequence"))
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::custom("unexpected number tuple"))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::custom("unexpected number tuple"))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::custom("unexpected number tuple"))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(Error::custom("unexpected number map"))
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Self::Error> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::custom("unexpected number variant"))
    }
}

pub(super) fn number_eq(a: &Number, b: &Number) -> bool {
    if a == b {
        return true;
    }
    if a.serialize(NumberEncoding).unwrap_or(true) {
        // Exact decimal arithmetic for arbitrary precision, consistently for every
        // number. Mixing decimal normalization with binary casts breaks transitivity.
        return normalized(a) == normalized(b);
    }
    let af = if a.is_f64() { a.as_f64() } else { None };
    let bf = if b.is_f64() { b.as_f64() } else { None };
    if let (Some(a), Some(b)) = (af, bf) {
        return a == b;
    }
    // Convert the float to an integer only after an exclusive upper-bound check. Casting
    // integers to f64 would incorrectly equate neighboring integers above 2^53.
    let mixed = match (af, bf) {
        (Some(float), None) => Some((float, b)),
        (None, Some(float)) => Some((float, a)),
        _ => None,
    };
    if let Some((float, integer)) = mixed {
        if let Some(value) = integer.as_u64() {
            return float.fract() == 0.0
                && (0.0..18_446_744_073_709_551_616.0).contains(&float)
                && float as u64 == value;
        }
        if let Some(value) = integer.as_i64() {
            return float.fract() == 0.0
                && (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&float)
                && float as i64 == value;
        }
    }
    normalized(a) == normalized(b)
}

fn normalized(number: &Number) -> (bool, String, String) {
    let text = number.to_string();
    let (negative, unsigned) = text
        .strip_prefix('-')
        .map_or((false, text.as_str()), |s| (true, s));
    let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let fraction = mantissa.split_once('.').map_or(0, |(_, f)| f.len());
    let digits = mantissa.replace('.', "");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return (false, String::new(), "0".to_owned());
    }
    let trimmed = digits.trim_end_matches('0');
    let shift = (digits.len() - trimmed.len()) as i128 - fraction as i128;
    (
        negative,
        trimmed.to_owned(),
        shifted_exponent(exponent, shift),
    )
}

// Arbitrary-precision JSON numbers can have exponents too large for any machine integer.
// Only a small mantissa-length adjustment is needed; decimal carry/borrow avoids parsing overflow.
fn shifted_exponent(exponent: &str, shift: i128) -> String {
    if let Ok(exponent) = exponent.parse::<i128>() {
        if let Some(sum) = exponent.checked_add(shift) {
            return sum.to_string();
        }
    }
    let negative = exponent.starts_with('-');
    let digits = exponent
        .trim_start_matches(['-', '+'])
        .trim_start_matches('0');
    let mut digits: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let adjustment = if negative { -shift } else { shift };
    let mut remaining = adjustment.unsigned_abs();
    let add = adjustment >= 0;
    for digit in digits.iter_mut().rev() {
        if remaining == 0 {
            break;
        }
        let amount = (remaining % 10) as u8;
        remaining /= 10;
        if add {
            let sum = *digit + amount;
            *digit = sum % 10;
            remaining += u128::from(sum >= 10);
        } else if *digit >= amount {
            *digit -= amount;
        } else {
            *digit = *digit + 10 - amount;
            remaining += 1;
        }
    }
    let mut prefix = String::new();
    if negative {
        prefix.push('-');
    }
    if remaining > 0 {
        prefix.push_str(&remaining.to_string());
    }
    let first = digits.iter().position(|d| *d != 0).unwrap_or(digits.len());
    prefix.extend(digits[first..].iter().map(|digit| char::from(b'0' + digit)));
    prefix
}
