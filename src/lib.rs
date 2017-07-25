
//! A semi-safe, semi-stateless wrapper around OpenGL 3.3 Core. This library provides various
//! utilities to make using OpenGL 3.3 safer. It uses rust's type system to encode some information
//! which helps prevent common errors. This library is primarily intended to be used for games,
//! but you can also use it to create other graphics applications.

#[cfg(feature = "serialize")]
extern crate serde;

extern crate gl;
extern crate png;
extern crate rusttype;

extern crate cable_math;

mod color;
mod input;
mod window;

pub mod texture;
#[macro_use]
pub mod shader;
pub mod buffer;
pub mod graphics;
pub mod framebuffer;
pub mod font;
pub mod draw_group;
//pub mod ui; // Temporarily disabled. Broken due to changes in font code. Should be rewritten to use draw_group

pub use color::*;
pub use input::*;
pub use window::*;

use std::time::{Instant, Duration};
use std::ops::{Add, Sub, AddAssign, SubAssign};

use cable_math::Vec2;

pub use draw_group::DrawGroup;

/// Utility to track time in a program
#[derive(Clone)]
pub struct Timer {
    start: Instant,
    last: Instant,
}

impl Timer {
    pub fn new() -> Timer {
        let now = Instant::now();

        Timer {
            start: now,
            last: now,
        }
    }

    /// Returns `(time_since_start, time_since_last_tick)`
    pub fn tick(&mut self) -> (Timing, Timing) {
        let now = Instant::now();

        let age = (now - self.start).into();
        let delta = (now - self.last).into();

        self.last = now;

        (age, delta)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timing(pub u64); 

impl Timing {
    pub fn zero() -> Timing { Timing(0) }

    pub fn from_ms(ms: u64) -> Timing { Timing(ms * 1_000_000) } 
    pub fn from_secs(s: u64) -> Timing { Timing(s * 1_000_000_000) } 
    pub fn from_secs_float(s: f32) -> Timing { Timing((s * 1_000_000_000.0) as u64) } 

    /// Converts this timing to seconds, truncating any overflow. 1.999 ms will be converted to 1 ms.
    pub fn as_ms(self) -> u64 { self.0 / 1_000_000 }

    /// Converts this timing to seconds, truncating any overflow. 1.999 seconds will be converted to 1.
    pub fn as_secs(self) -> u64 { self.0 / 1_000_000_000 }

    pub fn as_secs_float(self) -> f32 { self.0 as f32 / 1_000_000_000.0 }

    pub fn max(self, other: Timing) -> Timing {
        ::std::cmp::max(self, other)
    }

    pub fn min(self, other: Timing) -> Timing {
        ::std::cmp::min(self, other)
    }
}

impl Add for Timing {
    type Output = Timing;
    fn add(self, rhs: Timing) -> Timing {
        Timing(self.0 + rhs.0)
    }
}

impl Sub for Timing {
    type Output = Timing;
    fn sub(self, rhs: Timing) -> Timing {
        Timing(self.0 - rhs.0)
    }
}

impl AddAssign for Timing {
    fn add_assign(&mut self, rhs: Timing) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Timing {
    fn sub_assign(&mut self, rhs: Timing) {
        self.0 -= rhs.0;
    }
}

impl From<Duration> for Timing {
    fn from(d: Duration) -> Timing {
        Timing(d.as_secs()*1_000_000_000 + (d.subsec_nanos() as u64))
    }
}

impl From<Timing> for Duration {
    fn from(t: Timing) -> Duration {
        let nanos = t.0 % 1_000_000_000;
        let secs = t.0 / 1_000_000_000;
        Duration::new(secs, nanos as u32)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Region {
    pub min: Vec2<f32>,
    pub max: Vec2<f32>,
}

impl Region {
    pub fn center(&self) -> Vec2<f32> { (self.min + self.max) / 2.0 } 

    pub fn width(&self) -> f32        { self.max.x - self.min.x }
    pub fn height(&self) -> f32       { self.max.y - self.min.y }

    pub fn size(&self) -> Vec2<f32>   { self.max - self.min }

    /// Checks if the given point is inside this region.
    pub fn contains(&self, p: Vec2<f32>) -> bool {
        p.x > self.min.x && p.x < self.max.x &&
        p.y > self.min.y && p.y < self.max.y
    }

    /// Width divided by height.
    pub fn aspect(&self) -> f32 {
        let size = self.size();
        size.x / size.y
    }

    /// Swaps `min` and `max` along the y axis
    pub fn flip_y(self) -> Region {
        Region {
            min: Vec2::new(self.min.x, self.max.y),
            max: Vec2::new(self.max.x, self.min.y),
        }
    }

    /// Swaps `min` and `max` along the x axis
    pub fn flip_x(self) -> Region {
        Region {
            min: Vec2::new(self.max.x, self.min.y),
            max: Vec2::new(self.min.x, self.max.y),
        }
    }

    /// Returns the region in which this region overlaps the given other region. This might produce
    /// a negative region.
    pub fn overlap(self, other: Region) -> Region {
        Region {
            min: Vec2 {
                x: f32::max(self.min.x, other.min.x),
                y: f32::max(self.min.y, other.min.y),
            },
            max: Vec2 {
                x: f32::min(self.max.x, other.max.x),
                y: f32::min(self.max.y, other.max.y),
            },
        }
    }

    /// Moves `min` to `(0, 0)` but preserves width and height. 
    pub fn unpositioned(self) -> Region {
        Region {
            min: Vec2::zero(),
            max: self.max - self.min,
        }
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn top_right(self) -> Vec2<f32> {
        Vec2::new(self.min.y, self.max.x)
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn bottom_left(self) -> Vec2<f32> {
        Vec2::new(self.max.y, self.min.x)
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn top_left(self) -> Vec2<f32> {
        self.min
    }

    /// Assumes that `min` is top left and `max` bottom right
    pub fn bottom_right(self) -> Vec2<f32> {
        self.max
    }
}
