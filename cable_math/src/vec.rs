
use num::*;
use std::fmt;
use std::ops::*;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(C)]
pub struct Vec2<T: Copy> {
    pub x: T,
    pub y: T
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(C)]
pub struct Vec3<T: Copy> {
    pub x: T,
    pub y: T,
    pub z: T
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(C)]
pub struct Vec4<T: Copy> {
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

    /// Calculates the 2D cross product of the given vectors. This is equal
    /// the `z` component of the 3D cross product of two 3D vectors with the
    /// same `x` and `y` components, and with `z = 0`.
    ///
    /// Algebraically, the cross product is equal to `a.x*b.y - b.x*a.y`.
    ///
    /// The 2D cross product is mostly used to determine whether two vectors
    /// are clockwise or counterclockwise from one another. If the crossproduct
    /// is positive, the shortest rotation from `a` to `b` is counterclockwise.
    pub fn cross(a: Vec2<T>, b: Vec2<T>) -> T {
        a.x*b.y - a.y*b.x
    }

    /// Rotates this vector 90 degrees (π/2 radians) counterclockwise
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    /// let a = Vec2::new(3, 2);
    /// assert_eq!(Vec2::new(-2, 3), a.left());
    /// ```
    pub fn left(self) -> Vec2<T> {
        Vec2::new(T::zero() - self.y, self.x)
    }

    /// Rotates this vector 90 degrees (π/2 radians) clockwise
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    /// let a = Vec2::new(3, 2);
    /// assert_eq!(Vec2::new(2, -3), a.right());
    /// ```
    pub fn right(self) -> Vec2<T> {
        Vec2::new(self.y, T::zero() - self.x)
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
    /// let angle = 3.1415 / 4.0; // π/4 = 45°
    ///
    /// let epsilon = (a.angle() - angle).abs();
    ///
    /// assert!(epsilon < 0.001);
    /// ```
    pub fn angle(&self) -> T {
        (self.y / self.x).atan()
    }

    /// Rotates this vector counterclockwise by the given angle in radians.
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    ///
    /// let a = Vec2::new(1.0f32, 1.0);
    ///
    /// let b = a.rotate(3.1415); // π radians counterclockwise (Suffers from floating point errors)
    /// let c = a.left().left();  // π/2 radians counterclockwise, twice (Very precice)
    ///
    /// let error = (b - c).len();
    /// assert!(error < 0.0002); // Could get more precice with more digits of π
    /// ```
    pub fn rotate(&self, angle: T) -> Vec2<T> {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec2 {
            x: self.x*cos - self.y*sin, 
            y: self.x*sin + self.y*cos,
        }
    }

    /// Calculates the length of this vector
    pub fn len(&self) -> T {
        (self.x*self.x + self.y*self.y).sqrt()
    }

    /// Normalizes this vector, returning a new vector with a length of 1
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    /// let a = Vec2::new(4.0, 9.0);
    /// assert_eq!(1.0, a.normalize().len());
    /// ```
    pub fn normalize(self) -> Self {
        let len = self.len();
        Vec2 {
            x: self.x / len,
            y: self.y / len
        }
    }
}
impl<T: Float> Vec3<T> {
    /// Calculates the length of this vector
    pub fn len(&self) -> T {
        (self.x*self.x + self.y*self.y + self.z*self.z).sqrt()
    }

    /// Normalizes this vector, returning a new vector with a length of 1
    /// # Example
    /// ```
    /// use cable_math::Vec3;
    /// let a = Vec3::new(4.0, 9.0, 2.0);
    /// assert_eq!(1.0, a.normalize().len());
    /// ```
    pub fn normalize(self) -> Self {
        let len = self.len();
        Vec3 {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len
        }
    }
}
impl<T: Float> Vec4<T> {
    /// Calculates the length of this vector
    pub fn len(&self) -> T {
        (self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w).sqrt()
    }

    /// Normalizes this vector, returning a new vector with a length of 1
    /// # Example
    /// ```
    /// use cable_math::Vec4;
    /// let a = Vec4::new(4.0, 9.0, 2.0, 1.0);
    /// assert_eq!(1.0, a.normalize().len());
    /// ```
    pub fn normalize(self) -> Self {
        let len = self.len();
        Vec4 {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
            w: self.w / len
        }
    }
}

// Swizzling
impl<T: Num + Copy> Vec3<T> {
    /// Equal to `Vec3::new(vec.x, vec.y, z)`
    pub fn from2(vec: Vec2<T>, z: T) -> Vec3<T> { Vec3 { x: vec.x, y: vec.y, z: z } }
    /// Equal to `Vec2::new(vec.x, vec.y)`.
    pub fn xy(self) -> Vec2<T> { Vec2 { x: self.x, y: self.y } }
    /// Equal to `Vec2::new(vec.x, vec.z)`.
    pub fn xz(self) -> Vec2<T> { Vec2 { x: self.x, y: self.z } }
    /// Equal to `Vec2::new(vec.y, vec.z)`.
    pub fn yz(self) -> Vec2<T> { Vec2 { x: self.y, y: self.z } }
}
impl<T: Num + Copy> Vec4<T> {
    /// Equal to `Vec4::new(vec.x, vec.y, vec.z, w)`
    pub fn from3(vec: Vec3<T>, w: T) -> Vec4<T> { Vec4 { x: vec.x, y: vec.y, z: vec.z, w: w } }
    /// Equal to `Vec4::new(vec.x, vec.y, z, w)`
    pub fn from2(vec: Vec2<T>, z: T, w: T) -> Vec4<T> { Vec4 { x: vec.x, y: vec.y, z: z, w: w } }
    /// Equal to `Vec4::new(vec.x, vec.y, vec.z)`
    pub fn xyz(self) -> Vec3<T> { Vec3 { x: self.x, y: self.y, z: self.z } }
    /// Equal to `Vec2::new(vec.x, vec.y)`.
    pub fn xy(self) -> Vec2<T> { Vec2 { x: self.x, y: self.y } }
    /// Equal to `Vec2::new(vec.x, vec.z)`.
    pub fn xz(self) -> Vec2<T> { Vec2 { x: self.x, y: self.z } }
    /// Equal to `Vec2::new(vec.y, vec.z)`.
    pub fn yz(self) -> Vec2<T> { Vec2 { x: self.y, y: self.z } }
}

// Addition, subtraction and scaling
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

impl<T: Num + Copy> Mul<T> for Vec2<T> {
    type Output = Self; 
    fn mul(self, scalar: T) -> Self {
        Vec2 {
            x: self.x * scalar,
            y: self.y * scalar
        }
    }
}
impl<T: Num + Copy> MulAssign<T> for Vec2<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.x = self.x * scalar;
        self.y = self.y * scalar;
    }
}
impl<T: Num + Copy> Mul<T> for Vec3<T> {
    type Output = Self; 
    fn mul(self, scalar: T) -> Self {
        Vec3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar
        }
    }
}
impl<T: Num + Copy> MulAssign<T> for Vec3<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.x = self.x * scalar;
        self.y = self.y * scalar;
        self.z = self.z * scalar;
    }
}
impl<T: Num + Copy> Mul<T> for Vec4<T> {
    type Output = Self; 
    fn mul(self, scalar: T) -> Self {
        Vec4 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar
        }
    }
}
impl<T: Num + Copy> MulAssign<T> for Vec4<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.x = self.x * scalar;
        self.y = self.y * scalar;
        self.z = self.z * scalar;
        self.w = self.w * scalar;
    }
}

impl<T: Num + Copy> Div<T> for Vec2<T> {
    type Output = Self; 
    fn div(self, scalar: T) -> Self {
        Vec2 {
            x: self.x / scalar,
            y: self.y / scalar
        }
    }
}
impl<T: Num + Copy> DivAssign<T> for Vec2<T> {
    fn div_assign(&mut self, scalar: T) {
        self.x = self.x / scalar;
        self.y = self.y / scalar;
    }
}
impl<T: Num + Copy> Div<T> for Vec3<T> {
    type Output = Self; 
    fn div(self, scalar: T) -> Self {
        Vec3 {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar
        }
    }
}
impl<T: Num + Copy> DivAssign<T> for Vec3<T> {
    fn div_assign(&mut self, scalar: T) {
        self.x = self.x / scalar;
        self.y = self.y / scalar;
        self.z = self.z / scalar;
    }
}
impl<T: Num + Copy> Div<T> for Vec4<T> {
    type Output = Self; 
    fn div(self, scalar: T) -> Self {
        Vec4 {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
            w: self.w / scalar
        }
    }
}
impl<T: Num + Copy> DivAssign<T> for Vec4<T> {
    fn div_assign(&mut self, scalar: T) {
        self.x = self.x / scalar;
        self.y = self.y / scalar;
        self.z = self.z / scalar;
        self.w = self.w / scalar;
    }
}

impl<T: Num + Copy> Neg for Vec2<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Vec2 { x: T::zero()-self.x, y: T::zero()-self.y }
    }
}
impl<T: Num + Copy> Neg for Vec3<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Vec3 { x: T::zero()-self.x, y: T::zero()-self.y, z: T::zero()-self.z }
    }
}
impl<T: Num + Copy> Neg for Vec4<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Vec4 { x: T::zero()-self.x, y: T::zero()-self.y, z: T::zero()-self.z, w: T::zero()-self.w }
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

    #[test]
    fn scale() {
        let a = Vec3::new(1.0, 3.5, 7.3);
        assert_eq!(a.len() * 2.0, (a*2.0).len());

        let a = Vec2::polar(2.0, 3.1415) * 0.5;
        let b = Vec2::new(-1.0, 0.0);
        let dif = (a - b).len();
        assert!(dif < 0.0001);

        let mut a = Vec4::new(3, 4, 1, 2);
        a *= 2;
        assert_eq!(Vec4::new(6, 8, 2, 4), a);
    }
}

