
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

impl <T: Num + Copy + Signed> Vec2<T> {
    /// Makes all components positive
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    /// let a = Vec2::new(-3, 2);
    /// assert_eq!(Vec2::new(3, 2), a.abs());
    /// ```
    pub fn abs(self) -> Self {
        Vec2 { x: self.x.abs(), y: self.y.abs() }
    }
}
impl <T: Num + Copy + Signed> Vec3<T> {
    /// Makes all components positive
    /// # Example
    /// ```
    /// use cable_math::Vec3;
    /// let a = Vec3::new(-3, 2, -1);
    /// assert_eq!(Vec3::new(3, 2, 1), a.abs());
    /// ```
    pub fn abs(self) -> Self {
        Vec3 { x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }
}
impl <T: Num + Copy + Signed> Vec4<T> {
    /// Makes all components positive
    /// # Example
    /// ```
    /// use cable_math::Vec4;
    /// let a = Vec4::new(-3, 2, -1, 7);
    /// assert_eq!(Vec4::new(3, 2, 1, 7), a.abs());
    /// ```
    pub fn abs(self) -> Self {
        Vec4 { x: self.x.abs(), y: self.y.abs(), z: self.z.abs(), w: self.w.abs() }
    }
}

impl<T: Float> Vec2<T> {
    /// Constructs a vector from polar format. Takes a length and an angle
    /// in radians.
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    ///
    /// let a = Vec2::polar(1.0, 3.1415 / 4.0); // π/4 = 45°
    /// let b = Vec2::new(0.707, 0.707); // 0.707 is approx. 2.0.sqrt() / 2.0 
    /// let dif = (a - b).len();
    ///
    /// assert!(dif < 0.0002);
    /// ```
    pub fn polar(radius: T, angle: T) -> Vec2<T> {
        Vec2 {
            x: radius * angle.cos(),
            y: radius * angle.sin()
        }
    }

    /// Finds the direction in which this direction is pointing. Returns a
    /// angle in radians.
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    ///
    /// let a = Vec2::new(1.0f32, 1.0);
    /// let angle = 3.1315 / 4.0; // π/4 = 45°
    ///
    /// let epsilon = (a.angle() - angle).abs();
    ///
    /// assert!(epsilon < 0.003);
    /// ```
    pub fn angle(&self) -> T {
        (self.y / self.x).atan()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let a = Vec2::new(1, 2);
        let b = Vec2::new(4, -3);

        assert_eq!(a, a + Vec2::zero());
        assert_eq!(b, b + Vec2::zero());

        assert_eq!(Vec2::new(5, -1), a + b);

        let mut c = a;
        c += Vec2::new(1, 1);
        assert_eq!(Vec2::new(2, 3), c);

        c += c;
        assert_eq!(Vec2::new(4, 6), c);
    }

    #[test]
    fn subtraction() {
        let a = Vec2::new(1, 2);
        let b = Vec2::new(4, -3);

        assert_eq!(a, a - Vec2::zero());
        assert_eq!(b, b - Vec2::zero());

        assert_eq!(Vec2::new(-3, 5), a - b);

        let mut c = a;
        c -= Vec2::new(2, 1);
        assert_eq!(Vec2::new(-1, 1), c);

        c -= c;
        assert_eq!(Vec2::new(0, 0), c);
    }

    #[test]
    fn len() {
        assert_eq!(0.0, Vec2::<f32>::zero().len());

        let a = Vec2::new(4, 4);
        let b = Vec2::new(4.0, -3.0);

        assert_eq!(32, a.len_sqr());
        assert_eq!(5.0, b.len());
    }

    #[test]
    fn dot() {
        assert_eq!(0.0, Vec2::dot(Vec2::zero(), Vec2::zero()));
        assert_eq!(0.0, Vec3::dot(Vec3::zero(), Vec3::zero()));
        assert_eq!(0.0, Vec4::dot(Vec4::zero(), Vec4::zero()));

        let a = Vec3::new(0.0, 1.0, 1.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        assert_eq!(0.0, Vec3::dot(a, b));

        assert_eq!(14, Vec4::dot(Vec4::new(1, 3, 2, 5), Vec4::new(-1, 3, -2, 2)));
    }
}

