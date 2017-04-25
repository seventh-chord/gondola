
//! A color type, with utility methods for modifying colors and parsing colors from hex integers and strings. 

use gl;
use gl::types::*;
use std;
use shader::UniformValue;
use buffer::VertexData;

/// A color with red, green, blue and alpha components. All components are expected to be
/// between 0 and 1, both inclusinve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Creates a new, completly opaque (alpha = 1), color.
    ///
    /// All parameters are clamped so that they are between 0 and 1, both inclusive.
    pub fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color {
            r: clamp(r, 0.0, 1.0),
            g: clamp(g, 0.0, 1.0),
            b: clamp(b, 0.0, 1.0),
            a: 1.0
        }
    }

    /// Creates a new color.
    ///
    /// All parameters are clamped so that they are between 0 and 1, both inclusive.
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color {
            r: clamp(r, 0.0, 1.0),
            g: clamp(g, 0.0, 1.0),
            b: clamp(b, 0.0, 1.0),
            a: clamp(a, 0.0, 1.0)
        }
    }

    /// Creates a color from a hex string. The string should be of the format "#rrggbb" or
    /// "rrggbb", where each of r, g and b is a hexadecimal digit. Note that this currently does
    /// not support loading colors with a alpha channel. All colors created will be completly
    /// opaque.
    pub fn hex(string: &str) -> Option<Color> {
        let value = {
            if string.len() == 6 {
                u32::from_str_radix(string, 16)
            } else if string.len() == 7 {
                u32::from_str_radix(&string[1..], 16)
            } else {
                return None
            }
        };

        match value {
            Ok(value) => Some(Color::hex_int(value)),
            Err(_) =>    None,
        }
    }

    /// Creates a color from a hex int. Bit `0..8` (The eight least significant bits) are the
    /// red channel. Bit `8..16` are the green channel. Bit `16..24` are the blue channel. Note
    /// that this function currently ignores the alpha channel.
    ///
    /// # Example
    /// ```rust
    /// # use gondola::Color;
    /// let color = Color::hex_int(0xff00ff);
    ///
    /// assert_eq!(color, Color::rgb(1.0, 0.0, 1.0));
    /// ```
    pub fn hex_int(value: u32) -> Color {
        let r = value >> 16 & 0xff;
        let g = value >> 8 & 0xff;
        let b = value & 0xff;

        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;

        Color { r: r, g: g, b: b, a: 1.0 }
    }

    /// Converts this color to a hex string like "#ffa13b". Note that this function currently
    /// ignores the alpha channel.
    pub fn to_hex(&self) -> String {
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        let value = r << 16 | g << 8 | b;
        format!("#{:06x}", value)
    }

    /// Creates a new color based on this color, with the red, green and blue components multiplied
    /// by the given factor.
    pub fn with_lightness(&self, factor: f32) -> Color {
        Color {
            r: clamp(self.r*factor, 0.0, 1.0),
            g: clamp(self.g*factor, 0.0, 1.0),
            b: clamp(self.b*factor, 0.0, 1.0),
            a: self.a
        }
    }
}

// Does not properly handle NaN, which should not really matter
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        return min;
    }
    if value > max {
        return max;
    }
    value
}

impl VertexData for Color {
    type Primitive = f32;
    fn bytes() -> usize { std::mem::size_of::<f32>() * 4 }
    fn primitives() -> usize { 4 }
}
impl UniformValue for Color {
    unsafe fn set_uniform(color: &Color, location: GLint) {
        gl::Uniform4f(location, color.r, color.g, color.b, color.a);
    }

    unsafe fn set_uniform_slice(colors: &[Color], location: GLint) {
        gl::Uniform4fv(location, colors.len() as GLsizei, colors.as_ptr() as *const GLfloat);
    }
}

// Custom serialization
#[cfg(feature = "serialize")]
mod serialize {
    use super::*;

    use std::fmt;
    use serde::{Serialize, Deserialize, Serializer, Deserializer};
    use serde::de::{Visitor, Error};

    impl Serialize for Color {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(&self.to_hex())
        }
    }

    impl<'de> Deserialize<'de> for Color {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_str(ColorVisitor)
        }
    }

    struct ColorVisitor;
    impl<'de> Visitor<'de> for ColorVisitor {
        type Value = Color;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("A string representing a valid hex color")
        }

        fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
            match Color::hex(v) {
                Some(color) => Ok(color),
                None =>        Err(E::custom(format!("\"{}\" is not a valid color string", v))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hex() {
        assert_eq!("#ffa3b1", Color::hex("#ffa3b1").unwrap().to_hex());
        assert_eq!("#a300f1", Color::hex("#a300f1").unwrap().to_hex());
        assert_eq!("#000000", Color::hex("#000000").unwrap().to_hex());
        assert_eq!("#000001", Color::hex("#000001").unwrap().to_hex());
        assert_eq!("#100000", Color::hex("#100000").unwrap().to_hex());
    }
}

