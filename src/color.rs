
//! A color type, with utility methods for modifying colors and parsing colors from hex strings. 

use gl;
use gl::types::*;
use std;
use shader::UniformValue;
use buffer::{VertexData, DataType};

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
    /// All parameters should be between 0 and 1, both inclusive.
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
    /// All parameters should be between 0 and 1, both inclusive.
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color {
            r: clamp(r, 0.0, 1.0),
            g: clamp(g, 0.0, 1.0),
            b: clamp(b, 0.0, 1.0),
            a: clamp(a, 0.0, 1.0)
        }
    }

    /// Creates a color from a hex string. The string should be
    /// of the format "#rrggbb" or "rrggbb", where each of r, g
    /// and b is a hexadecimal digit.
    ///
    /// If the string is not valid, this returns solid white.
    pub fn hex(string: &str) -> Color {
        let value = {
            if string.len() == 6 {
                u32::from_str_radix(string, 16)
            } else if string.len() == 7 {
                u32::from_str_radix(&string[1..], 16)
            } else {
                Ok(0xffffff) // White
            }
        }.unwrap_or(0xffffff);

        Color::hex_int(value)
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

    /// Creates a new color based on this color, with the red,
    /// green and blue components multiplied by the given factor.
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
    fn bytes() -> usize { std::mem::size_of::<f32>() * 4 }
    fn primitives() -> usize { 4 }
    fn data_type() -> DataType { DataType::Float }
}
impl UniformValue for Color {
    unsafe fn set_uniform(&self, location: GLint) {
        gl::Uniform4f(location, self.r, self.g, self.b, self.a);
    }
}

