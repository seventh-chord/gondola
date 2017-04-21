
use std::fmt;
use std::marker::PhantomData;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::ser::SerializeTuple;
use serde::de::{Visitor, SeqAccess, Error};

use vec::*;

impl<T: Serialize> Serialize for Vec2<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut tuple = s.serialize_tuple(2)?;
        tuple.serialize_element(&self.x)?;
        tuple.serialize_element(&self.y)?;
        tuple.end()
    }
}
impl<T: Serialize> Serialize for Vec3<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut tuple = s.serialize_tuple(3)?;
        tuple.serialize_element(&self.x)?;
        tuple.serialize_element(&self.y)?;
        tuple.serialize_element(&self.z)?;
        tuple.end()
    }
}
impl<T: Serialize> Serialize for Vec4<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut tuple = s.serialize_tuple(4)?;
        tuple.serialize_element(&self.x)?;
        tuple.serialize_element(&self.y)?;
        tuple.serialize_element(&self.z)?;
        tuple.serialize_element(&self.w)?;
        tuple.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vec2<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_tuple(2, Vec2Visitor::new())
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vec3<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_tuple(3, Vec3Visitor::new())
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vec4<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_tuple(4, Vec4Visitor::new())
    }
}

struct Vec2Visitor<T>(PhantomData<T>);
impl<T> Vec2Visitor<T> {
    fn new() -> Self { Vec2Visitor(PhantomData) }
}
impl<'de, T: Deserialize<'de>> Visitor<'de> for Vec2Visitor<T> {
    type Value = Vec2<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A sequence of length 2")
    }

    fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error> 
        where A: SeqAccess<'de>,
    {
        let x: Option<T> = a.next_element()?;
        let y: Option<T> = a.next_element()?;
        let z: Option<T> = a.next_element()?;

        match (x, y, z) {
            (Some(x), Some(y), None) =>     Ok(Vec2::new(x, y)),
            (Some(_), None, None) =>        Err(A::Error::invalid_length(1, &"Array of length 2")),
            (Some(_), Some(_), Some(_)) =>  Err(A::Error::invalid_length(3, &"Array of length 2")),
            _ =>                            Err(A::Error::custom("Expected array of length 2, found nothing")),
        }
    }
}

struct Vec3Visitor<T>(PhantomData<T>);
impl<T> Vec3Visitor<T> {
    fn new() -> Self { Vec3Visitor(PhantomData) }
}
impl<'de, T: Deserialize<'de>> Visitor<'de> for Vec3Visitor<T> {
    type Value = Vec3<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A sequence of length 3")
    }

    fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>,
    {
        let x: Option<T> = a.next_element()?;
        let y: Option<T> = a.next_element()?;
        let z: Option<T> = a.next_element()?;
        let w: Option<T> = a.next_element()?;

        match (x, y, z, w) {
            (Some(x), Some(y), Some(z), None) =>    Ok(Vec3::new(x, y, z)),
            (Some(_), None, None, None) =>          Err(A::Error::invalid_length(1, &"Array of length 3")),
            (Some(_), Some(_), None, None) =>       Err(A::Error::invalid_length(2, &"Array of length 3")),
            (Some(_), Some(_), Some(_), Some(_)) => Err(A::Error::invalid_length(4, &"Array of length 3")),
            _ =>                                    Err(A::Error::custom("Expected array of length 3, found nothing")),
        }
    }
}

struct Vec4Visitor<T>(PhantomData<T>);
impl<T> Vec4Visitor<T> {
    fn new() -> Self { Vec4Visitor(PhantomData) }
}
impl<'de, T: Deserialize<'de>> Visitor<'de> for Vec4Visitor<T> {
    type Value = Vec4<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A sequence of length 4")
    }

    fn visit_seq<A>(self, mut a: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>,
    {
        let x: Option<T> = a.next_element()?;
        let y: Option<T> = a.next_element()?;
        let z: Option<T> = a.next_element()?;
        let w: Option<T> = a.next_element()?;
        let q: Option<T> = a.next_element()?;

        match (x, y, z, w, q) {
            (Some(x), Some(y), Some(z), Some(w), None) =>       Ok(Vec4::new(x, y, z, w)),
            (Some(_), None, None, None, None) =>                Err(A::Error::invalid_length(1, &"Array of length 4")),
            (Some(_), Some(_), None, None, None) =>             Err(A::Error::invalid_length(2, &"Array of length 4")),
            (Some(_), Some(_), Some(_), None, None) =>          Err(A::Error::invalid_length(3, &"Array of length 4")),
            (Some(_), Some(_), Some(_), Some(_), Some(_)) =>    Err(A::Error::invalid_length(5, &"Array of length 4")),
            _ =>                                                Err(A::Error::custom("Expected array of length 4, found nothing")),
        }
    }
}
