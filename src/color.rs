
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Creates a new, completly opaque, color.
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
    pub fn hex(string: &str) -> Color {
        let value = {
            if string.len() == 6 {
                i32::from_str_radix(string, 16)
            } else if string.len() == 7 {
                i32::from_str_radix(&string[1..], 16)
            } else {
                Ok(0xffffff) // White
            }
        }.unwrap_or(0xffffff);

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

/// Does not properly handle NaN
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        return min;
    }
    if value > max {
        return max;
    }
    value
}

