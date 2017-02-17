
use num::*;
use std::fmt;
use std::ops::*;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2<T: Num + Copy> {
    pub x: T,
    pub y: T
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec3<T: Num + Copy> {
    pub x: T,
    pub y: T,
    pub z: T
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec4<T: Num + Copy> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T
}

// General functions
impl<T: Num + Copy> Vec2<T> {
    /// Creates a new vector with the given components
    pub fn new(x: T, y: T) -> Vec2<T> { Vec2 { x: x, y: y } }
    /// Creates a new vector with all components set to 0
    pub fn zero() -> Vec2<T> { Vec2 { x: T::zero(), y: T::zero() } }

    /// Calculates the length of this vector, raised to the power of two.
    /// Note that this is cheaper than computing the actual length, as it
    /// does not require a `sqrt()`
    pub fn len_sqr(&self) -> T {
        self.x*self.x + self.y*self.y
    }

    pub fn dot(a: Vec2<T>, b: Vec2<T>) -> T {
        a.x*b.x + a.y*b.y
    }
}
impl<T: Num + Copy> Vec3<T> {
    /// Creates a new vector with the given components
    pub fn new(x: T, y: T, z: T) -> Vec3<T> { Vec3 { x: x, y: y, z: z } }
    /// Creates a new vector with all components set to 0
    pub fn zero() -> Vec3<T> { Vec3 { x: T::zero(), y: T::zero(), z: T::zero() } }
    
    /// Calculates the length of this vector, raised to the power of two.
    /// Note that this is cheaper than computing the actual length, as it
    /// does not require a `sqrt()`
    pub fn len_sqr(&self) -> T {
        self.x*self.x + self.y*self.y + self.z*self.z
    }

    pub fn dot(a: Vec3<T>, b: Vec3<T>) -> T {
        a.x*b.x + a.y*b.y + a.z*b.z
    }
}
impl<T: Num + Copy> Vec4<T> {
    /// Creates a new vector with the given components
    pub fn new(x: T, y: T, z: T, w: T) -> Vec4<T> { Vec4 { x: x, y: y, z: z, w: w } }
    /// Creates a new vector with all components set to 0
    pub fn zero() -> Vec4<T> { Vec4 { x: T::zero(), y: T::zero(), z: T::zero(), w: T::zero() } }


    /// Calculates the length of this vector, raised to the power of two.
    /// Note that this is cheaper than computing the actual length, as it
    /// does not require a `sqrt()`
    pub fn len_sqr(&self) -> T {
        self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w
    }

    pub fn dot(a: Vec4<T>, b: Vec4<T>) -> T {
        a.x*b.x + a.y*b.y + a.z*b.z + a.w*b.w
    }
}

impl<T: Float> Vec2<T> {
    /// Calculates the length of this vector
    pub fn len(&self) -> T {
        (self.x*self.x + self.y*self.y).sqrt()
    }
}
impl<T: Float> Vec3<T> {
    /// Calculates the length of this vector
    pub fn len(&self) -> T {
        (self.x*self.x + self.y*self.y + self.z*self.z).sqrt()
    }
}
impl<T: Float> Vec4<T> {
    /// Calculates the length of this vector
    pub fn len(&self) -> T {
        (self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w).sqrt()
    }
}

// Addition and subtraction
impl<T: Num + Copy> Add for Vec2<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self { Vec2::new(self.x + other.x, self.y + other.y) }
}
impl<T: Num + Copy> Add for Vec3<T> {
    type Output = Self;
    fn add(self, other: Self) -> Vec3<T> { Vec3::new(self.x + other.x, self.y + other.y, self.z + other.z) }
}
impl<T: Num + Copy> Add for Vec4<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self { Vec4::new(self.x + other.x, self.y + other.y, self.z + other.z, self.w + other.w) }
}
impl<T: Num + Copy> Sub for Vec2<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Vec2::new(self.x - other.x, self.y - other.y) }
}
impl<T: Num + Copy> Sub for Vec3<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z) }
}
impl<T: Num + Copy> Sub for Vec4<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Vec4::new(self.x - other.x, self.y - other.y, self.z - other.z, self.w - other.w) }
}

impl<T: Num + Copy> AddAssign for Vec2<T> {
    fn add_assign(&mut self, other: Self) {
        self.x = self.x + other.x;
        self.y = self.y + other.y;
    }
}
impl<T: Num + Copy> AddAssign for Vec3<T> {
    fn add_assign(&mut self, other: Self) {
        self.x = self.x + other.x;
        self.y = self.y + other.y;
        self.z = self.z + other.z;
    }
}
impl<T: Num + Copy> AddAssign for Vec4<T> {
    fn add_assign(&mut self, other: Self) {
        self.x = self.x + other.x;
        self.y = self.y + other.y;
        self.z = self.z + other.z;
        self.w = self.w + other.w;
    }
}
impl<T: Num + Copy> SubAssign for Vec2<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x = self.x - other.x;
        self.y = self.y - other.y;
    }
}
impl<T: Num + Copy> SubAssign for Vec3<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x = self.x - other.x;
        self.y = self.y - other.y;
        self.z = self.z - other.z;
    }
}
impl<T: Num + Copy> SubAssign for Vec4<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x = self.x - other.x;
        self.y = self.y - other.y;
        self.z = self.z - other.z;
        self.w = self.w - other.w;
    }
}

// Printing
impl<T: fmt::Display + Num + Copy> fmt::Display for Vec2<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
impl<T: fmt::Display + Num + Copy> fmt::Display for Vec3<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}
impl<T: fmt::Display + Num + Copy> fmt::Display for Vec4<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}


