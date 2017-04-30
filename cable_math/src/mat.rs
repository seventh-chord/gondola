
use num::*;
use vec::{Vec3, Vec4};
use std::ops::*;

/// A matrix which is layed out in column major format in memory
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct Mat4<T> {
    // The order in which the components are defined here is inverse of
    // the way they are typically would be used, as the matrix should
    // be stored in collumn major format for opengl interoparability
    pub a11: T, pub a21: T, pub a31: T, pub a41: T,
    pub a12: T, pub a22: T, pub a32: T, pub a42: T,
    pub a13: T, pub a23: T, pub a33: T, pub a43: T,
    pub a14: T, pub a24: T, pub a34: T, pub a44: T,
}

impl<T: Copy> Copy for Mat4<T> {}

impl<T: Num + Copy> Default for Mat4<T> {
    fn default() -> Mat4<T> {
        Mat4::identity()
    }
}

impl<T: Num + Copy> Mat4<T> {
    /// Creates a new Mat4 with all values set to 0
    pub fn zero() -> Mat4<T> {
        Mat4 {
            a11: T::zero(), a12: T::zero(), a13: T::zero(), a14: T::zero(),
            a21: T::zero(), a22: T::zero(), a23: T::zero(), a24: T::zero(),
            a31: T::zero(), a32: T::zero(), a33: T::zero(), a34: T::zero(),
            a41: T::zero(), a42: T::zero(), a43: T::zero(), a44: T::zero(),
        }
    }

    /// Creates a new identity matrix
    pub fn identity() -> Mat4<T> {
        Mat4 {
            a11: T::one(),  a12: T::zero(), a13: T::zero(), a14: T::zero(),
            a21: T::zero(), a22: T::one(),  a23: T::zero(), a24: T::zero(),
            a31: T::zero(), a32: T::zero(), a33: T::one(),  a34: T::zero(),
            a41: T::zero(), a42: T::zero(), a43: T::zero(), a44: T::one(),
        }
    }

    /// Creates a new matrix with the given values. The values are specified
    /// row by row.
    pub fn with_values(a11: T, a12: T, a13: T, a14: T,
                       a21: T, a22: T, a23: T, a24: T,
                       a31: T, a32: T, a33: T, a34: T,
                       a41: T, a42: T, a43: T, a44: T)
                       -> Mat4<T> {
        Mat4 {
            a11: a11, a12: a12, a13: a13, a14: a14,
            a21: a21, a22: a22, a23: a23, a24: a24,
            a31: a31, a32: a32, a33: a33, a34: a34,
            a41: a41, a42: a42, a43: a43, a44: a44
        }
    }

    /// Creates a new matrix from a 4x4 array.
    pub fn from_col_nested(data: [[T; 4]; 4]) -> Mat4<T> {
        Mat4 {
            a11: data[0][0], a12: data[0][1], a13: data[0][2], a14: data[0][3],
            a21: data[1][0], a22: data[1][1], a23: data[1][2], a24: data[1][3],
            a31: data[2][0], a32: data[2][1], a33: data[2][2], a34: data[2][3],
            a41: data[3][0], a42: data[3][1], a43: data[3][2], a44: data[3][3],
        }
    }

    /// Creates a new matrix from a flat array
    pub fn from_row_flat(data: [T; 16]) -> Mat4<T> {
        Mat4 {
            a11: data[0],  a12: data[1],  a13: data[2],  a14: data[3],
            a21: data[4],  a22: data[5],  a23: data[6],  a24: data[7],
            a31: data[8],  a32: data[9],  a33: data[10], a34: data[11],
            a41: data[12], a42: data[13], a43: data[14], a44: data[15],
        }
    }

    /// Converts the given quaterion to a matrix.
    pub fn from_quaternion(x: T, y: T, z: T, w: T) -> Mat4<T> {
        let zero = T::zero();
        let one = T::one();
        let two = one + one;

        Mat4 {
            a11: one - two*y*y - two*z*z,
            a12: two*x*y - two*z*w,
            a13: two*x*z + two*y*w,
            a14: zero,
            a21: two*x*y + two*w*z,
            a22: one - two*x*x - two*z*z,
            a23: two*y*z - two*w*x,
            a24: zero,
            a31: two*x*z - two*w*y,
            a32: two*y*z + two*w*x,
            a33: one - two*x*x - two*y*y,
            a34: zero,
            a41: zero, a42: zero, a43: zero, a44: one,
        }
    }

    /// Transposes this matrix, mirroring all its values along the diagonal.
    pub fn transpose(self) -> Mat4<T> {
        Mat4 {
            a11: self.a11, a12: self.a21, a13: self.a31, a14: self.a41,
            a21: self.a12, a22: self.a22, a23: self.a32, a24: self.a42,
            a31: self.a13, a32: self.a23, a33: self.a33, a34: self.a43,
            a41: self.a14, a42: self.a24, a43: self.a34, a44: self.a44
        }
    }

    /// Calculates the determinant of this matrix.
    pub fn determinant(&self) -> T {
        // What a mess :/
        self.a11*self.a22*self.a33*self.a44 + self.a11*self.a23*self.a34*self.a42 + self.a11*self.a24*self.a32*self.a43
        + self.a12*self.a21*self.a34*self.a43 + self.a12*self.a23*self.a31*self.a44 + self.a12*self.a24*self.a33*self.a41
        + self.a13*self.a21*self.a32*self.a44 + self.a13*self.a22*self.a34*self.a41 + self.a13*self.a24*self.a31*self.a42
        + self.a14*self.a21*self.a33*self.a42 + self.a14*self.a22*self.a31*self.a43 + self.a14*self.a23*self.a32*self.a41
        - self.a11*self.a22*self.a34*self.a43 - self.a11*self.a23*self.a32*self.a44 - self.a11*self.a24*self.a33*self.a42
        - self.a12*self.a21*self.a33*self.a44 - self.a12*self.a23*self.a34*self.a41 - self.a12*self.a24*self.a31*self.a43
        - self.a13*self.a21*self.a34*self.a42 - self.a13*self.a22*self.a31*self.a44 - self.a13*self.a24*self.a32*self.a41
        - self.a14*self.a21*self.a32*self.a43 - self.a14*self.a22*self.a33*self.a41 - self.a14*self.a23*self.a31*self.a42
    }
    
    /// Inverses this matrix, such that this matrix multiplied by its inverse will allways be the
    /// identity matrix.
    ///
    /// This operation is not defined for matricies whose determinant is 0. If the determinant of
    /// this matrix is 0 this function will panic.
    ///
    /// Note that due to floating point imprecissions, `A⁻¹A = I` (Where A is any matrix which can
    /// be inversed, and I is the identity matrix) will not usually be true. However, the
    /// difference is usually so small that it is negligible.
    pub fn inverse(self) -> Mat4<T> {
        let det = self.determinant();
        if det == T::zero() {
            panic!("Determinant of matrix is 0. Inverse is not defined");
        }

        // What a mess :/ :/ :/
        Mat4 {
            a11: self.a22*self.a33*self.a44 + self.a23*self.a34*self.a42 + self.a24*self.a32*self.a43 - self.a22*self.a34*self.a43 - self.a23*self.a32*self.a44 - self.a24*self.a33*self.a42,
            a12: self.a12*self.a34*self.a43 + self.a13*self.a32*self.a44 + self.a14*self.a33*self.a42 - self.a12*self.a33*self.a44 - self.a13*self.a34*self.a42 - self.a14*self.a32*self.a43,
            a13: self.a12*self.a23*self.a44 + self.a13*self.a24*self.a42 + self.a14*self.a22*self.a43 - self.a12*self.a24*self.a43 - self.a13*self.a22*self.a44 - self.a14*self.a23*self.a42,
            a14: self.a12*self.a24*self.a33 + self.a13*self.a22*self.a34 + self.a14*self.a23*self.a32 - self.a12*self.a23*self.a34 - self.a13*self.a24*self.a32 - self.a14*self.a22*self.a33,
            a21: self.a21*self.a34*self.a43 + self.a23*self.a31*self.a44 + self.a24*self.a33*self.a41 - self.a21*self.a33*self.a44 - self.a23*self.a34*self.a41 - self.a24*self.a31*self.a43,
            a22: self.a11*self.a33*self.a44 + self.a13*self.a34*self.a41 + self.a14*self.a31*self.a43 - self.a11*self.a34*self.a43 - self.a13*self.a31*self.a44 - self.a14*self.a33*self.a41,
            a23: self.a11*self.a24*self.a43 + self.a13*self.a21*self.a44 + self.a14*self.a23*self.a41 - self.a11*self.a23*self.a44 - self.a13*self.a24*self.a41 - self.a14*self.a21*self.a43,
            a24: self.a11*self.a23*self.a34 + self.a13*self.a24*self.a31 + self.a14*self.a21*self.a33 - self.a11*self.a24*self.a33 - self.a13*self.a21*self.a34 - self.a14*self.a23*self.a31,
            a31: self.a21*self.a32*self.a44 + self.a22*self.a34*self.a41 + self.a24*self.a31*self.a42 - self.a21*self.a34*self.a42 - self.a22*self.a31*self.a44 - self.a24*self.a32*self.a41,
            a32: self.a11*self.a34*self.a42 + self.a12*self.a31*self.a44 + self.a14*self.a32*self.a41 - self.a11*self.a32*self.a44 - self.a12*self.a34*self.a41 - self.a14*self.a31*self.a42,
            a33: self.a11*self.a22*self.a44 + self.a12*self.a24*self.a41 + self.a14*self.a21*self.a42 - self.a11*self.a24*self.a42 - self.a12*self.a21*self.a44 - self.a14*self.a22*self.a41,
            a34: self.a11*self.a24*self.a32 + self.a12*self.a21*self.a34 + self.a14*self.a22*self.a31 - self.a11*self.a22*self.a34 - self.a12*self.a24*self.a31 - self.a14*self.a21*self.a32,
            a41: self.a21*self.a33*self.a42 + self.a22*self.a31*self.a43 + self.a23*self.a32*self.a41 - self.a21*self.a32*self.a43 - self.a22*self.a33*self.a41 - self.a23*self.a31*self.a42,
            a42: self.a11*self.a32*self.a43 + self.a12*self.a33*self.a41 + self.a13*self.a31*self.a42 - self.a11*self.a33*self.a42 - self.a12*self.a31*self.a43 - self.a13*self.a32*self.a41,
            a43: self.a11*self.a23*self.a42 + self.a12*self.a21*self.a43 + self.a13*self.a22*self.a41 - self.a11*self.a22*self.a43 - self.a12*self.a23*self.a41 - self.a13*self.a21*self.a42,
            a44: self.a11*self.a22*self.a33 + self.a12*self.a23*self.a31 + self.a13*self.a21*self.a32 - self.a11*self.a23*self.a32 - self.a12*self.a21*self.a33 - self.a13*self.a22*self.a31
        } * (T::one() / det)
    }

    /// Creates a new orthographic projection matrix.
    pub fn ortho(left: T, right: T, top: T, bottom: T, near: T, far: T) -> Mat4<T> {
        let two = T::one() + T::one();
        let a11 = two / (right-left);
        let a22 = two / (top-bottom);
        let a33 = two / (near-far);

        let a14 = T::zero() - ((right+left) / (right-left));
        let a24 = T::zero() - ((top+bottom) / (top-bottom));
        let a34 = T::zero() - ((far+near) / (far - near));

        Mat4 {
            a11: a11, a12: T::zero(), a13: T::zero(), a14: a14,
            a21: T::zero(), a22: a22, a23: T::zero(), a24: a24,
            a31: T::zero(), a32: T::zero(), a33: a33, a34: a34,
            a41: T::zero(), a42: T::zero(), a43: T::zero(), a44: T::one()
        }
    }

    /// Creates a translation matrix.
    pub fn translation(translation: Vec3<T>) -> Mat4<T> {
        Mat4 {
            a14: translation.x, a24: translation.y, a34: translation.z,
            .. Mat4::identity()
        }
    }

    /// Creates a translation matrix which translates only along the x axis.
    pub fn translation_x(x: T) -> Mat4<T> {
        Mat4 {
            a14: x,
            .. Mat4::identity()
        }
    }

    /// Creates a translation matrix which translates only along the y axis.
    pub fn translation_y(y: T) -> Mat4<T> {
        Mat4 {
            a24: y,
            .. Mat4::identity()
        }
    }

    /// Creates a translation matrix which translates only along the z axis.
    pub fn translation_z(z: T) -> Mat4<T> {
        Mat4 {
            a34: z,
            .. Mat4::identity()
        }
    }

    /// Creates a scaling matrix.
    pub fn scaling(scale: Vec3<T>) -> Mat4<T> {
        Mat4 {
            a11: scale.x, a22: scale.y, a33: scale.z,
            .. Mat4::identity()
        }
    } 
}

impl<T: Float + Copy> Mat4<T> {
    /// Creates a new perspective projection matrix. `fov` is the vertical field of view and should
    /// be in degrees.
    pub fn perspective(fov: T, aspect: T, near: T, far: T) -> Mat4<T> {
        let two = T::one() + T::one();
        let top = (fov / two).to_radians().tan() * near;
        let right = top * aspect;
        Mat4 {
            a11: near / right,
            a22: near / top,
            a33: -(far + near) / (far - near),
            a34: (-two*far*near) / (far - near),
            a43: -T::one(),
            .. Mat4::zero()
        }
    }

    /// Creates a matrix representing a counterclockwise rotation of `angle` radians
    /// around the x-axis
    pub fn rotation_x(angle: T) -> Mat4<T> {
        let sin = angle.sin();
        let cos = angle.cos();
        Mat4 {
            a22: cos, a23: -sin,
            a32: sin, a33: cos,
            .. Mat4::identity()
        }
    }

    /// Creates a matrix representing a counterclockwise rotation of `angle` radians
    /// around the y-axis
    pub fn rotation_y(angle: T) -> Mat4<T> {
        let sin = angle.sin();
        let cos = angle.cos();
        Mat4 {
            a11: cos, a13: sin,
            a31: -sin, a33: cos,
            .. Mat4::identity()
        }
    }

    /// Creates a matrix representing a counterclockwise rotation of `angle` radians
    /// around the z-axis
    pub fn rotation_z(angle: T) -> Mat4<T> {
        let sin = angle.sin();
        let cos = angle.cos();
        Mat4 {
            a11: cos, a12: -sin,
            a21: sin, a22: cos,
            .. Mat4::identity()
        }
    }
}

// Multiplication
impl<T: Num + Copy> Mul for Mat4<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let a = self;
        let b = other;

        Mat4 {
            a11: a.a11*b.a11 + a.a12*b.a21 + a.a13*b.a31 + a.a14*b.a41,
            a12: a.a11*b.a12 + a.a12*b.a22 + a.a13*b.a32 + a.a14*b.a42,
            a13: a.a11*b.a13 + a.a12*b.a23 + a.a13*b.a33 + a.a14*b.a43,
            a14: a.a11*b.a14 + a.a12*b.a24 + a.a13*b.a34 + a.a14*b.a44,

            a21: a.a21*b.a11 + a.a22*b.a21 + a.a23*b.a31 + a.a24*b.a41,
            a22: a.a21*b.a12 + a.a22*b.a22 + a.a23*b.a32 + a.a24*b.a42,
            a23: a.a21*b.a13 + a.a22*b.a23 + a.a23*b.a33 + a.a24*b.a43,
            a24: a.a21*b.a14 + a.a22*b.a24 + a.a23*b.a34 + a.a24*b.a44,

            a31: a.a31*b.a11 + a.a32*b.a21 + a.a33*b.a31 + a.a34*b.a41,
            a32: a.a31*b.a12 + a.a32*b.a22 + a.a33*b.a32 + a.a34*b.a42,
            a33: a.a31*b.a13 + a.a32*b.a23 + a.a33*b.a33 + a.a34*b.a43,
            a34: a.a31*b.a14 + a.a32*b.a24 + a.a33*b.a34 + a.a34*b.a44,

            a41: a.a41*b.a11 + a.a42*b.a21 + a.a43*b.a31 + a.a44*b.a41,
            a42: a.a41*b.a12 + a.a42*b.a22 + a.a43*b.a32 + a.a44*b.a42,
            a43: a.a41*b.a13 + a.a42*b.a23 + a.a43*b.a33 + a.a44*b.a43,
            a44: a.a41*b.a14 + a.a42*b.a24 + a.a43*b.a34 + a.a44*b.a44,
        }
    }
}

// Scaling
impl<T: Num + Copy> MulAssign for Mat4<T> {
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}
impl<T: Num + Copy> Mul<T> for Mat4<T> {
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Mat4 {
            a11: self.a11 * scalar, a12: self.a12 * scalar, a13: self.a13 * scalar, a14: self.a14 * scalar,
            a21: self.a21 * scalar, a22: self.a22 * scalar, a23: self.a23 * scalar, a24: self.a24 * scalar,
            a31: self.a31 * scalar, a32: self.a32 * scalar, a33: self.a33 * scalar, a34: self.a34 * scalar,
            a41: self.a41 * scalar, a42: self.a42 * scalar, a43: self.a43 * scalar, a44: self.a44 * scalar,
        }
    }
}
impl<T: Num + Copy> MulAssign<T> for Mat4<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.a11 = self.a11 * scalar; self.a12 = self.a12 * scalar; self.a13 = self.a13 * scalar; self.a14 = self.a14 * scalar;
        self.a21 = self.a21 * scalar; self.a22 = self.a22 * scalar; self.a23 = self.a23 * scalar; self.a24 = self.a24 * scalar;
        self.a31 = self.a31 * scalar; self.a32 = self.a32 * scalar; self.a33 = self.a33 * scalar; self.a34 = self.a34 * scalar;
        self.a41 = self.a41 * scalar; self.a42 = self.a42 * scalar; self.a43 = self.a43 * scalar; self.a44 = self.a44 * scalar;
    }
}

// Vector multiplication
impl<T: Num + Copy> Mul<Vec4<T>> for Mat4<T> {
    type Output = Vec4<T>;
    fn mul(self, v: Vec4<T>) -> Vec4<T> {
        Vec4 {
            x: self.a11*v.x + self.a12*v.y + self.a13*v.z + self.a14*v.w,
            y: self.a21*v.x + self.a22*v.y + self.a23*v.z + self.a24*v.w,
            z: self.a31*v.x + self.a32*v.y + self.a33*v.z + self.a34*v.w,
            w: self.a41*v.x + self.a42*v.y + self.a43*v.z + self.a44*v.w,
        }
    }
}

// Addition and subtraction
impl<T: Num + Copy> Add for Mat4<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Mat4 {
            a11: self.a11 + other.a11, a12: self.a12 + other.a12, a13: self.a13 + other.a13, a14: self.a14 + other.a14,
            a21: self.a21 + other.a21, a22: self.a22 + other.a22, a23: self.a23 + other.a23, a24: self.a24 + other.a24,
            a31: self.a31 + other.a31, a32: self.a32 + other.a32, a33: self.a33 + other.a33, a34: self.a34 + other.a34,
            a41: self.a41 + other.a41, a42: self.a42 + other.a42, a43: self.a43 + other.a43, a44: self.a44 + other.a44,
        }
    }
}
impl<T: Num + Copy> AddAssign for Mat4<T> {
    fn add_assign(&mut self, other: Self) {
        self.a11 = self.a11 + other.a11; self.a12 = self.a12 + other.a12; self.a13 = self.a13 + other.a13; self.a14 = self.a14 + other.a14;
        self.a21 = self.a21 + other.a21; self.a22 = self.a22 + other.a22; self.a23 = self.a23 + other.a23; self.a24 = self.a24 + other.a24;
        self.a31 = self.a31 + other.a31; self.a32 = self.a32 + other.a32; self.a33 = self.a33 + other.a33; self.a34 = self.a34 + other.a34;
        self.a41 = self.a41 + other.a41; self.a42 = self.a42 + other.a42; self.a43 = self.a43 + other.a43; self.a44 = self.a44 + other.a44;
    }
}
impl<T: Num + Copy> Sub for Mat4<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Mat4 {
            a11: self.a11 - other.a11, a12: self.a12 - other.a12, a13: self.a13 - other.a13, a14: self.a14 - other.a14,
            a21: self.a21 - other.a21, a22: self.a22 - other.a22, a23: self.a23 - other.a23, a24: self.a24 - other.a24,
            a31: self.a31 - other.a31, a32: self.a32 - other.a32, a33: self.a33 - other.a33, a34: self.a34 - other.a34,
            a41: self.a41 - other.a41, a42: self.a42 - other.a42, a43: self.a43 - other.a43, a44: self.a44 - other.a44,
        }
    }
}
impl<T: Num + Copy> SubAssign for Mat4<T> {
    fn sub_assign(&mut self, other: Self) {
        self.a11 = self.a11 - other.a11; self.a12 = self.a12 - other.a12; self.a13 = self.a13 - other.a13; self.a14 = self.a14 - other.a14;
        self.a21 = self.a21 - other.a21; self.a22 = self.a22 - other.a22; self.a23 = self.a23 - other.a23; self.a24 = self.a24 - other.a24;
        self.a31 = self.a31 - other.a31; self.a32 = self.a32 - other.a32; self.a33 = self.a33 - other.a33; self.a34 = self.a34 - other.a34;
        self.a41 = self.a41 - other.a41; self.a42 = self.a42 - other.a42; self.a43 = self.a43 - other.a43; self.a44 = self.a44 - other.a44;
    }
}

impl<T: Num + Copy> AsRef<[T]> for Mat4<T> {
    fn as_ref(&self) -> &[T] {
        use std::slice;
        unsafe {
            slice::from_raw_parts(&self.a11 as *const T, 16)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Function for generating random testing matrices
    fn mat_a() -> Mat4<f32> {
        Mat4::with_values(1.0, 7.0, 4.0, 3.0,
                          5.0, 6.0, 7.0, 8.0,
                          9.0, 2.0, 3.0, 1.0,
                          6.0, 6.0, 2.0, 7.0)
    }
    fn mat_b() -> Mat4<f32> {
        Mat4::with_values(7.0, 8.0, 2.0, 9.0,
                          1.0, 3.0, 5.0, 2.0,
                          3.0, 6.0, 3.0, 7.0,
                          2.0, 7.0, 3.0, 8.0)
    }
    fn mat_c() -> Mat4<f32> {
        Mat4::with_values(5.0, 7.0, 1.0, 2.0,
                          3.0, 1.0, 8.0, 4.0,
                          9.0, 2.0, 0.0, 1.0,
                          3.0, 7.0, 1.0, 8.0)
    }

    #[test]
    fn identity() {
        let a = mat_a();
        let b = mat_b();
        let c = mat_c();
        let identity = Mat4::identity();

        assert_eq!(a, a * identity);
        assert_eq!(b, b * identity);
        assert_eq!(c, c * identity);

        assert_eq!(a, identity * a);
        assert_eq!(b, identity * b);
        assert_eq!(c, identity * c);

        assert_ne!(identity * a, identity * b);
        assert_ne!(identity * b, identity * c);
        assert_ne!(identity * a, identity * c);

    }

    #[test]
    fn zero() {
        let a = mat_a();
        let b = mat_b();
        let c = mat_c();
        let identity = Mat4::identity();
        let zero = Mat4::zero();

        assert_eq!(identity * zero, zero);

        assert_eq!(zero, a * zero);
        assert_eq!(zero, b * zero);
        assert_eq!(zero, c * zero);
    }

    #[test]
    fn mul() {
        let a = mat_a();
        let b = mat_b();
        let ab = Mat4::with_values(32.0, 74.0, 58.0, 75.0,
                                   78.0, 156.0, 85.0, 170.0,
                                   76.0, 103.0, 40.0, 114.0,
                                   68.0, 127.0, 69.0, 136.0);

        assert_eq!(ab, a * b);

        let mut c = a;
        c *= b;

        assert_eq!(ab, c);

        let ba = Mat4::with_values(119.0, 155.0, 108.0, 150.0,
                                   73.0, 47.0, 44.0, 46.0,
                                   102.0, 105.0, 77.0, 109.0,
                                   112.0, 110.0, 82.0, 121.0);
        assert_eq!(ba, b * a);

        assert_ne!(b * a, a * b);
    }

    #[test]
    fn scale() {
        let a = Mat4::with_values(1, 2, 3, 4,
                                  5, 6, 7, 8,
                                  1, 2, 3, 4,
                                  5, 6, 7, 8);
        let b = Mat4::with_values(2, 4, 6, 8,
                                  10, 12, 14, 16,
                                  2, 4, 6, 8,
                                  10, 12, 14, 16);
        let mut c = a;
        c *= 2;

        assert_eq!(b, a*2);
        assert_eq!(c, a*2);
        assert_eq!(c, b);
    }

    #[test]
    fn add() {
        let a = mat_a();
        let b = mat_b();
        let sum = Mat4::with_values(8.0, 15.0, 6.0, 12.0,
                                    6.0, 9.0, 12.0, 10.0,
                                    12.0, 8.0, 6.0, 8.0,
                                    8.0, 13.0, 5.0, 15.0);
        assert_eq!(sum, a + b);
    }

    #[test]
    fn sub() {
        let a = mat_a();
        let b = mat_b();
        let dif = Mat4::with_values(-6.0, -1.0, 2.0, -6.0,
                                    4.0, 3.0, 2.0, 6.0,
                                    6.0, -4.0, 0.0, -6.0,
                                    4.0, -1.0, -1.0, -1.0);
        assert_eq!(dif, a - b);
    }

    #[test]
    fn transpose() {
        let a = mat_a();
        let expected = Mat4::with_values(1.0, 5.0, 9.0, 6.0,
                                         7.0, 6.0, 2.0, 6.0,
                                         4.0, 7.0, 3.0, 2.0,
                                         3.0, 8.0, 1.0, 7.0);
        assert_eq!(expected, a.transpose());
    }

    #[test]
    fn determinant() {
        assert_eq!(1538.0, mat_a().determinant());
        assert_eq!(61.0, mat_b().determinant());
    }

    #[test]
    fn inverse() {
        let identity = Mat4::<f32>::identity();
        let inverse = identity.inverse();

        assert_eq!(identity, inverse);

        let i_det = identity.determinant();

        let a_det = (mat_a()*mat_a().inverse()).determinant();
        let b_det = (mat_b()*mat_b().inverse()).determinant();
        let c_det = (mat_c()*mat_c().inverse()).determinant();

        assert!((i_det - a_det).abs() < 0.00001);
        assert!((i_det - b_det).abs() < 0.00001);
        assert!((i_det - c_det).abs() < 0.00001);
    }

    #[test]
    fn vec_mul() {
        let identity = Mat4::<f32>::identity();
        let vec = Vec4::new(7.2, 2.4, 3.4, 1.9);

        assert_eq!(vec, identity * vec);
    }

    #[test]
    fn inv_vec_mul() {
        let vec = Vec4::new(6.3, -1.3, 4.3, -2.8);

        let a = mat_a();
        let result = a * (a.inverse() * vec);
        assert!((vec - result).len() < 0.00001);

        let b = mat_b();
        let result = b * (b.inverse() * vec);
        assert!((vec - result).len() < 0.00001);

        let c = mat_c();
        let result = c * (c.inverse() * vec);
        assert!((vec - result).len() < 0.00001);
    }
}

