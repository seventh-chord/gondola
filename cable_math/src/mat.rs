
use num::*;
use vec::Vec3;
use std::ops::*;

/// A matrix which is layed out in column major format in memory
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Mat4<T: Num + Copy> {
    // The order in which the components are defined here is inverse of
    // the way they are typically would be used, as the matrix should
    // be stored in collumn major format for opengl interoparability
    pub a11: T, pub a21: T, pub a31: T, pub a41: T,
    pub a12: T, pub a22: T, pub a32: T, pub a42: T,
    pub a13: T, pub a23: T, pub a33: T, pub a43: T,
    pub a14: T, pub a24: T, pub a34: T, pub a44: T,
}

impl<T: Num + Copy> Mat4<T> {
    /// Creates a new Mat4 with all values set to 0
    pub fn new() -> Mat4<T> {
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

    /// Creates a new orthographic projection matrix
    pub fn ortho(left: T, right: T, top: T, bottom: T, near: T, far: T) -> Mat4<T> {
        let two = T::one() + T::one();
        let a11 = two / (right-left);
        let a22 = two / (top-left);
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

    /// Creates a translation matrix
    pub fn translation(translation: Vec3<T>) -> Mat4<T> {
        Mat4 {
            a14: translation.x, a24: translation.y, a34: translation.z,
            .. Mat4::identity()
        }
    }
}

// Multiplication (Ugly code)
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
impl<T: Num + Copy> MulAssign for Mat4<T> {
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}
