
use std::fmt;
use std::marker::PhantomData;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::ser::{SerializeSeq};
use serde::de::{Visitor, SeqVisitor, Error};

use vec::*;

impl<T: Serialize> Serialize for Vec2<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq_fixed_size(2)?;
        seq.serialize_element(&self.x)?;
        seq.serialize_element(&self.y)?;
        seq.end()
    }
}
impl<T: Serialize> Serialize for Vec3<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq_fixed_size(3)?;
        seq.serialize_element(&self.x)?;
        seq.serialize_element(&self.y)?;
        seq.serialize_element(&self.z)?;
        seq.end()
    }
}
impl<T: Serialize> Serialize for Vec4<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq_fixed_size(4)?;
        seq.serialize_element(&self.x)?;
        seq.serialize_element(&self.y)?;
        seq.serialize_element(&self.z)?;
        seq.serialize_element(&self.w)?;
        seq.end()
    }
}

impl<T: Deserialize> Deserialize for Vec2<T> {
    fn deserialize<D: Deserializer>(d: D) -> Result<Self, D::Error> {
        d.deserialize_seq_fixed_size(2, Vec2Visitor::new())
    }
}
impl<T: Deserialize> Deserialize for Vec3<T> {
    fn deserialize<D: Deserializer>(d: D) -> Result<Self, D::Error> {
        d.deserialize_seq_fixed_size(3, Vec3Visitor::new())
    }
}
impl<T: Deserialize> Deserialize for Vec4<T> {
    fn deserialize<D: Deserializer>(d: D) -> Result<Self, D::Error> {
        d.deserialize_seq_fixed_size(4, Vec4Visitor::new())
    }
}

struct Vec2Visitor<T>(PhantomData<T>);
impl<T> Vec2Visitor<T> {
    fn new() -> Self { Vec2Visitor(PhantomData) }
}
impl<T: Deserialize> Visitor for Vec2Visitor<T> {
    type Value = Vec2<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A sequence of length 2")
    }

    fn visit_seq<V: SeqVisitor>(self, mut v: V) -> Result<Self::Value, V::Error> {
        let x: Option<T> = v.visit()?;
        let y: Option<T> = v.visit()?;
        let z: Option<T> = v.visit()?;

        match (x, y, z) {
            (Some(x), Some(y), None) =>     Ok(Vec2::new(x, y)),
            (Some(_), None, None) =>        Err(V::Error::invalid_length(1, &"Array of length 2")),
            (Some(_), Some(_), Some(_)) =>  Err(V::Error::invalid_length(3, &"Array of length 2")),
            _ =>                            Err(V::Error::custom("Expected array of length 2, found nothing")),
        }
    }
}

struct Vec3Visitor<T>(PhantomData<T>);
impl<T> Vec3Visitor<T> {
    fn new() -> Self { Vec3Visitor(PhantomData) }
}
impl<T: Deserialize> Visitor for Vec3Visitor<T> {
    type Value = Vec3<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A sequence of length 3")
    }

    fn visit_seq<V: SeqVisitor>(self, mut v: V) -> Result<Self::Value, V::Error> {
        let x: Option<T> = v.visit()?;
        let y: Option<T> = v.visit()?;
        let z: Option<T> = v.visit()?;
        let w: Option<T> = v.visit()?;

        match (x, y, z, w) {
            (Some(x), Some(y), Some(z), None) =>    Ok(Vec3::new(x, y, z)),
            (Some(_), None, None, None) =>          Err(V::Error::invalid_length(1, &"Array of length 3")),
            (Some(_), Some(_), None, None) =>       Err(V::Error::invalid_length(2, &"Array of length 3")),
            (Some(_), Some(_), Some(_), Some(_)) => Err(V::Error::invalid_length(4, &"Array of length 3")),
            _ =>                                    Err(V::Error::custom("Expected array of length 3, found nothing")),
        }
    }
}

struct Vec4Visitor<T>(PhantomData<T>);
impl<T> Vec4Visitor<T> {
    fn new() -> Self { Vec4Visitor(PhantomData) }
}
impl<T: Deserialize> Visitor for Vec4Visitor<T> {
    type Value = Vec4<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A sequence of length 4")
    }

    fn visit_seq<V: SeqVisitor>(self, mut v: V) -> Result<Self::Value, V::Error> {
        let x: Option<T> = v.visit()?;
        let y: Option<T> = v.visit()?;
        let z: Option<T> = v.visit()?;
        let w: Option<T> = v.visit()?;
        let q: Option<T> = v.visit()?;

        match (x, y, z, w, q) {
            (Some(x), Some(y), Some(z), Some(w), None) =>       Ok(Vec4::new(x, y, z, w)),
            (Some(_), None, None, None, None) =>                Err(V::Error::invalid_length(1, &"Array of length 4")),
            (Some(_), Some(_), None, None, None) =>             Err(V::Error::invalid_length(2, &"Array of length 4")),
            (Some(_), Some(_), Some(_), None, None) =>          Err(V::Error::invalid_length(3, &"Array of length 4")),
            (Some(_), Some(_), Some(_), Some(_), Some(_)) =>    Err(V::Error::invalid_length(5, &"Array of length 4")),
            _ =>                                                Err(V::Error::custom("Expected array of length 4, found nothing")),
        }
    }
}
