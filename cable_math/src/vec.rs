
use std::fmt;
use std::ops::{Add, Sub, Mul, Div};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use std::ops::Neg;

use traits::{Number, Float, Signed};

#[derive(Debug, Clone, PartialEq, Default)]
#[repr(C)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T
}

#[derive(Debug, Clone, PartialEq, Default)]
#[repr(C)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T
}

#[derive(Debug, Clone, PartialEq, Default)]
#[repr(C)]
pub struct Vec4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T
}

// Copy
impl<T: Copy> Copy for Vec2<T> {}
impl<T: Copy> Copy for Vec3<T> {}
impl<T: Copy> Copy for Vec4<T> {}

// Constructors
impl<T> Vec2<T> {
    /// Creates a new vector with the given components
    pub fn new(x: T, y: T) -> Vec2<T> { Vec2 { x: x, y: y } }
}
impl<T> Vec3<T> {
    /// Creates a new vector with the given components
    pub fn new(x: T, y: T, z: T) -> Vec3<T> { Vec3 { x: x, y: y, z: z } }
}
impl<T> Vec4<T> {
    /// Creates a new vector with the given components
    pub fn new(x: T, y: T, z: T, w: T) -> Vec4<T> { Vec4 { x: x, y: y, z: z, w: w } }
}

// General functions
impl<T: Number> Vec2<T> {
    pub const ZERO: Vec2<T> = Vec2 { x: T::ZERO, y: T::ZERO };
    pub const X: Vec2<T> = Vec2 { x: T::ONE, y: T::ZERO };
    pub const Y: Vec2<T> = Vec2 { x: T::ZERO, y: T::ONE };

    /// Calculates the length of this vector, raised to the power of two.
    /// Note that this is cheaper than computing the actual length, as it
    /// does not require a `sqrt()`
    pub fn len_sqr(&self) -> T {
        self.x*self.x + self.y*self.y
    }

    pub fn dot(a: Vec2<T>, b: Vec2<T>) -> T {
        a.x*b.x + a.y*b.y
    }

    pub fn componentwise_multiply(a: Vec2<T>, b: Vec2<T>) -> Vec2<T> {
        Vec2::new(a.x*b.x, a.y*b.y)
    }

    pub fn componentwise_divide(a: Vec2<T>, b: Vec2<T>) -> Vec2<T> {
        Vec2::new(a.x/b.x, a.y/b.y)
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
        Vec2::new(T::ZERO - self.y, self.x)
    }

    /// Rotates this vector 90 degrees (π/2 radians) clockwise
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    /// let a = Vec2::new(3, 2);
    /// assert_eq!(Vec2::new(2, -3), a.right());
    /// ```
    pub fn right(self) -> Vec2<T> {
        Vec2::new(self.y, T::ZERO - self.x)
    }

    /// Projects this vector onto the given other vector. The returned vector will lie on a line
    /// going through the origin and `ray`. `ray` does not need to be normalized.
    pub fn project_onto(self, ray: Vec2<T>) -> Vec2<T> {
        // A more readable version of the below would be:
        // ray.normalize() * dot(self, ray.normalize())

        let dot = self.x*ray.x + self.y*ray.y;
        let len = ray.x*ray.x + ray.y*ray.y;
        ray * (dot / len)
    }

    /// Linearly interpolates between `a` and `b`. Normally `t` should be between 0 and 1 both
    /// inclusive, where 0 gives just `a` and 1 gives just `b`.
    pub fn lerp(a: Self, b: Self, t: T) -> Self {
        a*(T::ONE - t) + b*t
    }
}

impl<T: Number> Vec3<T> {
    pub const ZERO: Vec3<T> = Vec3 { x: T::ZERO, y: T::ZERO, z: T::ZERO };
    pub const X: Vec3<T> = Vec3 { x: T::ONE, y: T::ZERO, z: T::ZERO };
    pub const Y: Vec3<T> = Vec3 { x: T::ZERO, y: T::ONE, z: T::ZERO };
    pub const Z: Vec3<T> = Vec3 { x: T::ZERO, y: T::ZERO, z: T::ONE };
    
    /// Calculates the length of this vector, raised to the power of two.
    /// Note that this is cheaper than computing the actual length, as it
    /// does not require a `sqrt()`
    pub fn len_sqr(&self) -> T {
        self.x*self.x + self.y*self.y + self.z*self.z
    }

    pub fn dot(a: Vec3<T>, b: Vec3<T>) -> T {
        a.x*b.x + a.y*b.y + a.z*b.z
    }

    pub fn componentwise_multiply(a: Vec3<T>, b: Vec3<T>) -> Vec3<T> {
        Vec3::new(a.x*b.x, a.y*b.y, a.z*b.z)
    }

    pub fn componentwise_divide(a: Vec3<T>, b: Vec3<T>) -> Vec3<T> {
        Vec3::new(a.x/b.x, a.y/b.y, a.z/b.z)
    }

    pub fn cross(a: Vec3<T>, b: Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: a.y*b.z - a.z*b.y,
            y: a.z*b.x - a.x*b.z,
            z: a.x*b.y - a.y*b.x,
        }
    }

    /// Linearly interpolates between `a` and `b`. Normally `t` should be between 0 and 1 both
    /// inclusive, where 0 gives just `a` and 1 gives just `b`.
    pub fn lerp(a: Self, b: Self, t: T) -> Self {
        a*(T::ONE - t) + b*t
    }
}

impl<T: Number> Vec4<T> {
    pub const ZERO: Vec4<T> = Vec4 { x: T::ZERO, y: T::ZERO, z: T::ZERO, w: T::ZERO };
    pub const X: Vec4<T> = Vec4 { x: T::ONE, y: T::ZERO, z: T::ZERO, w: T::ZERO };
    pub const Y: Vec4<T> = Vec4 { x: T::ZERO, y: T::ONE, z: T::ZERO, w: T::ZERO };
    pub const Z: Vec4<T> = Vec4 { x: T::ZERO, y: T::ZERO, z: T::ONE, w: T::ZERO };
    pub const W: Vec4<T> = Vec4 { x: T::ZERO, y: T::ZERO, z: T::ZERO, w: T::ONE };

    /// Calculates the length of this vector, raised to the power of two.
    /// Note that this is cheaper than computing the actual length, as it
    /// does not require a `sqrt()`
    pub fn len_sqr(&self) -> T {
        self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w
    }

    pub fn dot(a: Vec4<T>, b: Vec4<T>) -> T {
        a.x*b.x + a.y*b.y + a.z*b.z + a.w*b.w
    }

    pub fn componentwise_multiply(a: Vec4<T>, b: Vec4<T>) -> Vec4<T> {
        Vec4::new(a.x*b.x, a.y*b.y, a.z*b.z, a.w*b.w)
    }

    pub fn componentwise_divide(a: Vec4<T>, b: Vec4<T>) -> Vec4<T> {
        Vec4::new(a.x/b.x, a.y/b.y, a.z/b.z, a.w/b.w)
    }
    
    /// Linearly interpolates between `a` and `b`. Normally `t` should be between 0 and 1 both
    /// inclusive, where 0 gives just `a` and 1 gives just `b`.
    pub fn lerp(a: Self, b: Self, t: T) -> Self {
        a*(T::ONE - t) + b*t
    }
}

impl <T: Signed> Vec2<T> {
    /// Makes all components positive
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    /// let a = Vec2::new(-3, 2);
    /// assert_eq!(Vec2::new(3, 2), a.abs());
    /// ```
    pub fn abs(self) -> Vec2<T> {
        Vec2 { x: self.x.abs(), y: self.y.abs() }
    }
}
impl <T: Signed> Vec3<T> {
    /// Makes all components positive
    /// # Example
    /// ```
    /// use cable_math::Vec3;
    /// let a = Vec3::new(-3, 2, -1);
    /// assert_eq!(Vec3::new(3, 2, 1), a.abs());
    /// ```
    pub fn abs(self) -> Vec3<T> {
        Vec3 { x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }
}
impl <T: Signed> Vec4<T> {
    /// Makes all components positive
    /// # Example
    /// ```
    /// use cable_math::Vec4;
    /// let a = Vec4::new(-3, 2, -1, 7);
    /// assert_eq!(Vec4::new(3, 2, 1, 7), a.abs());
    /// ```
    pub fn abs(self) -> Vec4<T> {
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
        self.y.atan2(self.x)
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
    pub fn normalize(self) -> Vec2<T> {
        let len = self.len();
        Vec2 {
            x: self.x / len,
            y: self.y / len
        }
    }

    /// Rounds all components of this vector to the nearest integer number.
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    ///
    /// let a = Vec2::new(3.4, 2.7);
    /// let b = Vec2::new(3.0, 3.0);
    ///
    /// assert_eq!(a.round(), b);
    pub fn round(self) -> Vec2<T> {
        Vec2 {
            x: self.x.round(),
            y: self.y.round(),
        }
    }

    /// Rounds all values of this vector down to the nearest integer multiple (Applying `floor`)
    pub fn floor(self) -> Vec2<T> {
        Vec2 {
            x: self.x.floor(),
            y: self.y.floor(),
        }
    }
    
    /// Rounds all values of this vector up to the nearest integer multiple (Applying `ceil`)
    pub fn ceil(self) -> Vec2<T> {
        Vec2 {
            x: self.x.ceil(),
            y: self.y.ceil(),
        }
    }

    /// Treating the two given vectors as complex numbers, with `x` being the real part and `y`
    /// being the imaginary part, this function algebraically multiplies the two values. Complex
    /// multiplication is commutative.
    ///
    /// In general, this function can be used to quickly rotate vectors: Multiplying (with complex
    /// multiplication) by a unit vector pointing in some angle is the same as rotating the other
    /// vector by that angle. [For more information][1].
    ///
    /// [1]: https://en.wikipedia.org/wiki/Imaginary_number#Geometric_interpretation
    ///
    /// # Example
    /// ```
    /// use cable_math::Vec2;
    ///
    /// let angle = 4.3; 
    /// let a = Vec2::polar(1.0, angle); 
    /// let b = Vec2::new(4.0, 5.0);
    ///
    /// let complexly_rotated = Vec2::complex_mul(a, b);
    /// let simply_rotated = b.rotate(angle);
    ///
    /// assert!((complexly_rotated - simply_rotated).len() < 0.001);
    /// ```
    pub fn complex_mul(a: Vec2<T>, b: Vec2<T>) -> Vec2<T> {
        Vec2 {
            x: a.x*b.x - a.y*b.y,
            y: a.x*b.y + a.y*b.x,
        }
    }

    /// Finds the complex transpose of this vector. This basically just changes the sign of the `y`
    /// comopnent. When used with `complex_mul`, the transpose of a vector yields the oposite
    /// rotation.
    pub fn transpose(self) -> Vec2<T> {
        Vec2 { x: self.x, y: -self.y }
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
    pub fn normalize(self) -> Vec3<T> {
        let len = self.len();
        Vec3 {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len
        }
    }

    /// Rounds all components of this vector to the nearest integer number.
    /// # Example
    /// ```
    /// use cable_math::Vec3;
    ///
    /// let a = Vec3::new(3.4, 2.7, -1.2);
    /// let b = Vec3::new(3.0, 3.0, -1.0);
    ///
    /// assert_eq!(a.round(), b);
    pub fn round(self) -> Vec3<T> {
        Vec3 {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
        }
    }

    /// Rounds all values of this vector down to the nearest integer multiple (Applying `floor`)
    pub fn floor(self) -> Vec3<T> {
        Vec3 {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor(),
        }
    }
    
    /// Rounds all values of this vector up to the nearest integer multiple (Applying `ceil`)
    pub fn ceil(self) -> Vec3<T> {
        Vec3 {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil(),
        }
    }

    /// Rotates this vector by the given amount of radians around the x-axis in the
    /// counter-clockwise direction, acroding to the right hand rule.
    ///
    /// This rotates through the quadrants in the following order: +y, +z, -y, -z.
    ///
    /// # Example
    /// ```
    /// use cable_math::Vec3;
    ///
    /// let a = Vec3::new(0.0, 0.0, 1.0); // +z
    /// let b = Vec3::new(0.0, -1.0, 0.0); // -y
    ///
    /// let dif = b - a.rotate_x(1.571); // Approximately π/2
    ///
    /// assert!(dif.len() < 0.001);
    /// ```
    pub fn rotate_x(self, angle: T) -> Vec3<T> {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec3 {
            x: self.x,
            y: self.y*cos - self.z*sin,
            z: self.y*sin + self.z*cos,
        }
    }

    /// Rotates this vector by the given amount of radians around the y-axis in the
    /// counter-clockwise direction, acording to the right hand rule.
    ///
    /// This rotates through the quadrants in the following order: +x, -z, -x, +z.
    ///
    /// # Example
    /// ```
    /// use cable_math::Vec3;
    ///
    /// let a = Vec3::new(0.0, 0.0, -1.0); // -z
    /// let b = Vec3::new(-1.0, 0.0, 0.0); // -x
    ///
    /// let dif = b - a.rotate_y(1.571); // Approximately π/2
    ///
    /// assert!(dif.len() < 0.001);
    /// ```
    pub fn rotate_y(self, angle: T) -> Vec3<T> {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec3 {
            x: self.x*cos + self.z*sin,
            y: self.y,
            z: -self.x*sin + self.z*cos,
        }
    }

    /// Rotates this vector by the given amount of radians around the z-axis in the
    /// counter-clockwise direction, acording to the right hand rule.
    ///
    /// This rotates through the quadrants in the following order: +x, +y, -x, -y
    ///
    /// # Example
    /// ```
    /// use cable_math::Vec3;
    ///
    /// let a = Vec3::new(-1.0, 0.0, 0.0); // -x
    /// let b = Vec3::new(0.0, -1.0, 0.0); // -y
    ///
    /// let dif = b - a.rotate_z(1.571); // Approximately π/2
    ///
    /// assert!(dif.len() < 0.001);
    /// ```
    pub fn rotate_z(self, angle: T) -> Vec3<T> {
        let cos = angle.cos();
        let sin = angle.sin();
        Vec3 {
            x: self.x*cos - self.y*sin,
            y: self.x*sin + self.y*cos,
            z: self.z,
        }
    }
}
impl<T: Float> Vec4<T> {
    /// Calculates the length of this vector.
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
    pub fn normalize(self) -> Vec4<T> {
        let len = self.len();
        Vec4 {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
            w: self.w / len
        }
    }

    /// Rounds all components of this vector to the nearest integer number.
    /// # Example
    /// ```
    /// use cable_math::Vec4;
    ///
    /// let a = Vec4::new(3.4, 2.7, -1.2, 0.1);
    /// let b = Vec4::new(3.0, 3.0, -1.0, 0.0);
    ///
    /// assert_eq!(a.round(), b);
    pub fn round(self) -> Vec4<T> {
        Vec4 {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
            w: self.w.round(),
        }
    }

    /// Rounds all values of this vector down to the nearest integer multiple (Applying `floor`)
    pub fn floor(self) -> Vec4<T> {
        Vec4 {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor(),
            w: self.w.floor(),
        }
    }
    
    /// Rounds all values of this vector up to the nearest integer multiple (Applying `ceil`)
    pub fn ceil(self) -> Vec4<T> {
        Vec4 {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil(),
            w: self.w.ceil(),
        }
    }
}

// Swizzling
impl<T: Number> Vec3<T> {
    /// Equal to `Vec3::new(vec.x, vec.y, z)`
    pub fn from2(vec: Vec2<T>, z: T) -> Vec3<T> { Vec3 { x: vec.x, y: vec.y, z: z } }
    /// Equal to `Vec2::new(vec.x, vec.y)`.
    pub fn xy(self) -> Vec2<T> { Vec2 { x: self.x, y: self.y } }
    /// Equal to `Vec2::new(vec.x, vec.z)`.
    pub fn xz(self) -> Vec2<T> { Vec2 { x: self.x, y: self.z } }
    /// Equal to `Vec2::new(vec.y, vec.z)`.
    pub fn yz(self) -> Vec2<T> { Vec2 { x: self.y, y: self.z } }
}
impl<T: Number> Vec4<T> {
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
impl<T: Number> Add for Vec2<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self { Vec2::new(self.x + other.x, self.y + other.y) }
}
impl<T: Number> Add for Vec3<T> {
    type Output = Self;
    fn add(self, other: Self) -> Vec3<T> { Vec3::new(self.x + other.x, self.y + other.y, self.z + other.z) }
}
impl<T: Number> Add for Vec4<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self { Vec4::new(self.x + other.x, self.y + other.y, self.z + other.z, self.w + other.w) }
}
impl<T: Number> Sub for Vec2<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Vec2::new(self.x - other.x, self.y - other.y) }
}
impl<T: Number> Sub for Vec3<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z) }
}
impl<T: Number> Sub for Vec4<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Vec4::new(self.x - other.x, self.y - other.y, self.z - other.z, self.w - other.w) }
}

impl<T: Number> AddAssign for Vec2<T> {
    fn add_assign(&mut self, other: Self) {
        self.x = self.x + other.x;
        self.y = self.y + other.y;
    }
}
impl<T: Number> AddAssign for Vec3<T> {
    fn add_assign(&mut self, other: Self) {
        self.x = self.x + other.x;
        self.y = self.y + other.y;
        self.z = self.z + other.z;
    }
}
impl<T: Number> AddAssign for Vec4<T> {
    fn add_assign(&mut self, other: Self) {
        self.x = self.x + other.x;
        self.y = self.y + other.y;
        self.z = self.z + other.z;
        self.w = self.w + other.w;
    }
}
impl<T: Number> SubAssign for Vec2<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x = self.x - other.x;
        self.y = self.y - other.y;
    }
}
impl<T: Number> SubAssign for Vec3<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x = self.x - other.x;
        self.y = self.y - other.y;
        self.z = self.z - other.z;
    }
}
impl<T: Number> SubAssign for Vec4<T> {
    fn sub_assign(&mut self, other: Self) {
        self.x = self.x - other.x;
        self.y = self.y - other.y;
        self.z = self.z - other.z;
        self.w = self.w - other.w;
    }
}

impl<T: Number> Mul<T> for Vec2<T> {
    type Output = Self; 
    fn mul(self, scalar: T) -> Self {
        Vec2 {
            x: self.x * scalar,
            y: self.y * scalar
        }
    }
}
impl<T: Number> MulAssign<T> for Vec2<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.x = self.x * scalar;
        self.y = self.y * scalar;
    }
}
impl<T: Number> Mul<T> for Vec3<T> {
    type Output = Self; 
    fn mul(self, scalar: T) -> Self {
        Vec3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar
        }
    }
}
impl<T: Number> MulAssign<T> for Vec3<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.x = self.x * scalar;
        self.y = self.y * scalar;
        self.z = self.z * scalar;
    }
}
impl<T: Number> Mul<T> for Vec4<T> {
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
impl<T: Number> MulAssign<T> for Vec4<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.x = self.x * scalar;
        self.y = self.y * scalar;
        self.z = self.z * scalar;
        self.w = self.w * scalar;
    }
}

impl<T: Number> Div<T> for Vec2<T> {
    type Output = Self; 
    fn div(self, scalar: T) -> Self {
        Vec2 {
            x: self.x / scalar,
            y: self.y / scalar
        }
    }
}
impl<T: Number> DivAssign<T> for Vec2<T> {
    fn div_assign(&mut self, scalar: T) {
        self.x = self.x / scalar;
        self.y = self.y / scalar;
    }
}
impl<T: Number> Div<T> for Vec3<T> {
    type Output = Self; 
    fn div(self, scalar: T) -> Self {
        Vec3 {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar
        }
    }
}
impl<T: Number> DivAssign<T> for Vec3<T> {
    fn div_assign(&mut self, scalar: T) {
        self.x = self.x / scalar;
        self.y = self.y / scalar;
        self.z = self.z / scalar;
    }
}
impl<T: Number> Div<T> for Vec4<T> {
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
impl<T: Number> DivAssign<T> for Vec4<T> {
    fn div_assign(&mut self, scalar: T) {
        self.x = self.x / scalar;
        self.y = self.y / scalar;
        self.z = self.z / scalar;
        self.w = self.w / scalar;
    }
}

impl<T: Number> Neg for Vec2<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Vec2 { x: T::ZERO-self.x, y: T::ZERO-self.y }
    }
}
impl<T: Number> Neg for Vec3<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Vec3 { x: T::ZERO-self.x, y: T::ZERO-self.y, z: T::ZERO-self.z }
    }
}
impl<T: Number> Neg for Vec4<T> {
    type Output = Self;
    fn neg(self) -> Self {
        Vec4 { x: T::ZERO-self.x, y: T::ZERO-self.y, z: T::ZERO-self.z, w: T::ZERO-self.w }
    }
}

// Printing
impl<T: fmt::Display + Number> fmt::Display for Vec2<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
impl<T: fmt::Display + Number> fmt::Display for Vec3<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}
impl<T: fmt::Display + Number> fmt::Display for Vec4<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

// Fake casting stuff
macro_rules! impl_cast {
    ($a:ty, $b:ty, $fn_name:ident) => {
        impl Vec2<$a> {
            pub fn $fn_name(self) -> Vec2<$b> {
                Vec2 { x: self.x as $b, y: self.y as $b }
            }
        }

        impl Vec3<$a> {
            pub fn $fn_name(self) -> Vec3<$b> {
                Vec3 { x: self.x as $b, y: self.y as $b, z: self.z as $b }
            }
        }

        impl Vec4<$a> {
            pub fn $fn_name(self) -> Vec4<$b> {
                Vec4 { x: self.x as $b, y: self.y as $b, z: self.z as $b, w: self.w as $b }
            }
        }
    };
}

impl_cast!(u8,  f32, as_f32);
impl_cast!(i8,  f32, as_f32);
impl_cast!(u16, f32, as_f32);
impl_cast!(i16, f32, as_f32);
impl_cast!(u32, f32, as_f32);
impl_cast!(i32, f32, as_f32);
impl_cast!(u64, f32, as_f32);
impl_cast!(i64, f32, as_f32);

impl_cast!(u8,  f64, as_f64);
impl_cast!(i8,  f64, as_f64);
impl_cast!(u16, f64, as_f64);
impl_cast!(i16, f64, as_f64);
impl_cast!(u32, f64, as_f64);
impl_cast!(i32, f64, as_f64);
impl_cast!(u64, f64, as_f64);
impl_cast!(i64, f64, as_f64);

impl_cast!(f32, i8,  as_i8);
impl_cast!(f64, i8,  as_i8); 
impl_cast!(f32, u8,  as_u8);
impl_cast!(f64, u8,  as_u8); 
impl_cast!(f32, i16, as_i16);
impl_cast!(f64, i16, as_i16); 
impl_cast!(f32, u16, as_u16);
impl_cast!(f64, u16, as_u16); 
impl_cast!(f32, i32, as_i32);
impl_cast!(f64, i32, as_i32); 
impl_cast!(f32, u32, as_u32);
impl_cast!(f64, u32, as_u32); 
impl_cast!(f32, i64, as_i64);
impl_cast!(f64, i64, as_i64); 
impl_cast!(f32, u64, as_u64);
impl_cast!(f64, u64, as_u64);

impl_cast!(f64, f32, as_f32);
impl_cast!(f32, f64, as_f64);

// Tuple to vector conversions
impl<T> From<(T, T)> for Vec2<T> {
    fn from((x, y): (T, T)) -> Vec2<T> {
        Vec2 { x, y }
    }
}
impl<T> From<(T, T, T)> for Vec3<T> {
    fn from((x, y, z): (T, T, T)) -> Vec3<T> {
        Vec3 { x, y, z }
    }
}
impl<T> From<(T, T, T, T)> for Vec4<T> {
    fn from((x, y, z, w): (T, T, T, T)) -> Vec4<T> {
        Vec4 { x, y, z, w }
    }
}

// Array to vector conversions
impl<T> From<[T; 2]> for Vec2<T> where T: Copy {
    fn from(a: [T; 2]) -> Vec2<T> {
        let (x, y) = (a[0], a[1]);
        Vec2 { x, y }
    }
}
impl<T> From<[T; 3]> for Vec3<T> where T: Copy {
    fn from(a: [T; 3]) -> Vec3<T> {
        let (x, y, z) = (a[0], a[1], a[2]);
        Vec3 { x, y, z }
    }
}
impl<T> From<[T; 4]> for Vec4<T> where T: Copy {
    fn from(a: [T; 4]) -> Vec4<T> {
        let (x, y, z, w) = (a[0], a[1], a[2], a[3]);
        Vec4 { x, y, z, w }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let a = Vec2::new(1, 2);
        let b = Vec2::new(4, -3);

        assert_eq!(a, a + Vec2::ZERO);
        assert_eq!(b, b + Vec2::ZERO);

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

        assert_eq!(a, a - Vec2::ZERO);
        assert_eq!(b, b - Vec2::ZERO);

        assert_eq!(Vec2::new(-3, 5), a - b);

        let mut c = a;
        c -= Vec2::new(2, 1);
        assert_eq!(Vec2::new(-1, 1), c);

        c -= c;
        assert_eq!(Vec2::new(0, 0), c);
    }

    #[test]
    fn len() {
        assert_eq!(0.0, Vec2::<f32>::ZERO.len());

        let a = Vec2::new(4, 4);
        let b = Vec2::new(4.0, -3.0);

        assert_eq!(32, a.len_sqr());
        assert_eq!(5.0, b.len());
    }

    #[test]
    fn dot() {
        assert_eq!(0.0, Vec2::dot(Vec2::ZERO, Vec2::ZERO));
        assert_eq!(0.0, Vec3::dot(Vec3::ZERO, Vec3::ZERO));
        assert_eq!(0.0, Vec4::dot(Vec4::ZERO, Vec4::ZERO));

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

