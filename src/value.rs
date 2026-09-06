use serde::ser::{self, Serialize};
use std::fmt;

pub(crate) enum Value {
    Empty,
    Atom(String),
    Struct(Vec<(String, Value)>),
    Seq(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Variant(String, Box<Value>),
}

impl Value {
    pub(crate) fn cell(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Atom(s) => clean(s),
            _ => self.inline(),
        }
    }

    fn inline(&self) -> String {
        match self {
            Self::Empty => "null".into(),
            Self::Atom(s) => {
                // Quote strings with delimiters or whitespace, including the empty string.
                if s.is_empty()
                    || s.chars()
                        .any(|c| c.is_whitespace() || c.is_control() || "\"\\,[]{}:()".contains(c))
                {
                    format!(
                        "\"{}\"",
                        s.chars().flat_map(char::escape_debug).collect::<String>()
                    )
                } else {
                    s.clone()
                }
            }
            Self::Struct(fields) => format!(
                "{{{}}}",
                fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", Value::Atom(k.clone()).inline(), v.inline()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Seq(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::inline)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Map(entries) => {
                let mut entries = entries
                    .iter()
                    .map(|(k, v)| (k.inline(), v.inline()))
                    .collect::<Vec<_>>();
                entries.sort();
                format!(
                    "{{{}}}",
                    entries
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::Variant(name, value) => {
                format!("{}({})", Self::Atom(name.clone()).inline(), value.inline())
            }
        }
    }
}

pub(crate) fn clean(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() && c != '\n' {
                c.escape_debug().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

#[derive(Debug)]
pub(crate) struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

pub(crate) struct Serializer;

macro_rules! atom {
    ($($method:ident($ty:ty)),* $(,)?) => {$ (
        fn $method(self, value: $ty) -> Result<Value, Error> { Ok(Value::Atom(value.to_string())) }
    )*};
}

impl ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = Sequence;
    type SerializeTuple = Sequence;
    type SerializeTupleStruct = Sequence;
    type SerializeTupleVariant = Sequence;
    type SerializeMap = Map;
    type SerializeStruct = Record;
    type SerializeStructVariant = Record;

    atom!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char),
        serialize_str(&str)
    );

    fn serialize_bytes(self, v: &[u8]) -> Result<Value, Error> {
        Ok(Value::Seq(
            v.iter().map(|b| Value::Atom(b.to_string())).collect(),
        ))
    }
    fn serialize_none(self) -> Result<Value, Error> {
        Ok(Value::Empty)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<Value, Error> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::Empty)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Value, Error> {
        Ok(Value::Empty)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        Ok(Value::Atom(variant.into()))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<Value, Error> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<Value, Error> {
        Ok(Value::Variant(variant.into(), Box::new(v.serialize(self)?)))
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Sequence, Error> {
        Ok(Sequence::new(None))
    }
    fn serialize_tuple(self, _: usize) -> Result<Sequence, Error> {
        self.serialize_seq(None)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Sequence, Error> {
        self.serialize_seq(None)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Sequence, Error> {
        Ok(Sequence::new(Some(variant)))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Map, Error> {
        Ok(Map {
            entries: Vec::new(),
            key: None,
        })
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Record, Error> {
        Ok(Record::new(None))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Record, Error> {
        Ok(Record::new(Some(variant)))
    }
}

pub(crate) struct Sequence {
    values: Vec<Value>,
    variant: Option<&'static str>,
}
impl Sequence {
    fn new(variant: Option<&'static str>) -> Self {
        Self {
            values: Vec::new(),
            variant,
        }
    }
    fn push<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.values.push(v.serialize(Serializer)?);
        Ok(())
    }
    fn finish(self) -> Value {
        let value = Value::Seq(self.values);
        match self.variant {
            Some(name) => Value::Variant(name.into(), Box::new(value)),
            None => value,
        }
    }
}
macro_rules! sequence {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for Sequence {
            type Ok = Value;
            type Error = Error;
            fn $method<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
                self.push(v)
            }
            fn end(self) -> Result<Value, Error> {
                Ok(self.finish())
            }
        }
    };
}
sequence!(SerializeSeq, serialize_element);
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);

pub(crate) struct Record {
    fields: Vec<(String, Value)>,
    variant: Option<&'static str>,
}
impl Record {
    fn new(variant: Option<&'static str>) -> Self {
        Self {
            fields: Vec::new(),
            variant,
        }
    }
    fn push<T: ?Sized + Serialize>(&mut self, key: &'static str, v: &T) -> Result<(), Error> {
        self.fields.push((key.into(), v.serialize(Serializer)?));
        Ok(())
    }
    fn finish(self) -> Value {
        let value = Value::Struct(self.fields);
        match self.variant {
            Some(name) => Value::Variant(name.into(), Box::new(value)),
            None => value,
        }
    }
}
macro_rules! record {
    ($trait:ident) => {
        impl ser::$trait for Record {
            type Ok = Value;
            type Error = Error;
            fn serialize_field<T: ?Sized + Serialize>(
                &mut self,
                key: &'static str,
                v: &T,
            ) -> Result<(), Error> {
                self.push(key, v)
            }
            fn end(self) -> Result<Value, Error> {
                Ok(self.finish())
            }
        }
    };
}
record!(SerializeStruct);
record!(SerializeStructVariant);

pub(crate) struct Map {
    entries: Vec<(Value, Value)>,
    key: Option<Value>,
}
impl ser::SerializeMap for Map {
    type Ok = Value;
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        if self.key.is_some() {
            return Err(Error("map key without a value".into()));
        }
        self.key = Some(key.serialize(Serializer)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error("map value without a key".into()))?;
        self.entries.push((key, value.serialize(Serializer)?));
        Ok(())
    }
    fn end(self) -> Result<Value, Error> {
        if self.key.is_some() {
            return Err(Error("map key without a value".into()));
        }
        Ok(Value::Map(self.entries))
    }
}
