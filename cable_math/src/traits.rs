
use {Vec2, Vec3, Vec4, Mat2, Mat3, Mat4, Quaternion};

use std::ops::{Add, Sub, Mul, Div};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use std::ops::{Neg};
use std::cmp::{PartialEq, PartialOrd};

/// Allows us to be generic over numeric types in vectors
pub trait Number: 
    Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> +
    AddAssign + SubAssign + MulAssign + DivAssign +
    PartialEq + PartialOrd +
    Sized + Copy
{
    const ONE: Self;
    const ZERO: Self;
}

/// Allows us to be generic over signed numeric types in vectors
pub trait Signed: Number + Neg<Output = Self> {
    #[inline(always)]
    fn abs(self) -> Self {
        if self < Self::ZERO {
            -self
        } else {
            self
        }
    }
}

macro_rules! impl_number {
    ($type: ident, $one: expr, $zero: expr) => {
        impl Number for $type {
            const ONE: Self = $one;
            const ZERO: Self = $zero;
        }
    };
}

impl_number!(i8,  1, 0);
impl_number!(i16, 1, 0);
impl_number!(i32, 1, 0);
impl_number!(i64, 1, 0);
impl_number!(u8,  1, 0);
impl_number!(u16, 1, 0);
impl_number!(u32, 1, 0);
impl_number!(u64, 1, 0);
impl_number!(f32, 1.0, 0.0);
impl_number!(f64, 1.0, 0.0);

impl Signed for i8 {}
impl Signed for i16 {}
impl Signed for i32 {}
impl Signed for i64 {}
impl Signed for f32 {}
impl Signed for f64 {}

macro_rules! impl_float {
    ($($fn: ident),*) => {
        /// Allows us to be generic over floating point types
        pub trait Float: Number + Signed + Round {
            $(fn $fn(self) -> Self;)*
            fn sin_cos(self) -> (Self, Self);
            fn atan2(self, other: Self) -> Self;
        }

        impl Float for f32 {
            $(#[inline(always)] fn $fn(self) -> Self { self.$fn() })*
            #[inline(always)]
            fn sin_cos(self) -> (Self, Self) { self.sin_cos() }
            #[inline(always)]
            fn atan2(self, other: Self) -> Self { self.atan2(other) }
        }

        impl Float for f64 {
            $(#[inline(always)] fn $fn(self) -> Self { self.$fn() })*
            #[inline(always)]
            fn sin_cos(self) -> (Self, Self) { self.sin_cos() }
            #[inline(always)]
            fn atan2(self, other: Self) -> Self { self.atan2(other) }
        }
    };
}

impl_float!(sin, cos, tan, asin, acos, atan, sqrt, floor, ceil, to_radians);

/// Provides convenience functions for rounding all memebers of math types in various ways.
pub trait Round {
    type Step: Copy;

    /// Rounds this value so that it has no decimal places.
    fn round(self) -> Self;

    /// Rounds the given value to the given number of decimals.
    fn round_to_precision(self, precision: usize) -> Self;

    /// Rounds the given value to the next multiple of `step`.
    fn round_to_step(self, step: Self::Step) -> Self;
}

impl Round for f32 {
    fn round(self) -> Self  {
        f32::round(self)
    }

    fn round_to_precision(self, precision: usize) -> Self {
        let s = 10f32.powi(precision as i32);
        (self * s).round() / s
    }

    type Step = f32;

    fn round_to_step(self, step: f32) -> Self {
        (self / step).round() * step
    }
}

impl Round for f64 { 
    fn round(self) -> Self  {
        f64::round(self)
    }

    fn round_to_precision(self, precision: usize) -> Self {
        let s = 10f64.powi(precision as i32);
        (self * s).round() / s
    }

    type Step = f64;

    fn round_to_step(self, step: f64) -> Self {
        (self / step).round() * step
    }
}

macro_rules! impl_round {
    ($ty: ty, [$($field: ident),*]) => {
        impl<T: Round> Round for $ty {
            fn round(self) -> Self { 
                Self {
                    $($field: self.$field.round()),*
                } 
            }

            fn round_to_precision(self, precision: usize) -> Self { 
                Self {
                    $($field: self.$field.round_to_precision(precision)),*
                } 
            }

            type Step = T::Step;

            fn round_to_step(self, step: T::Step) -> Self { 
                Self {
                    $($field: self.$field.round_to_step(step)),*
                } 
            }
        }
    };
}

impl_round!(Vec2<T>, [x, y]);
impl_round!(Vec3<T>, [x, y, z]);
impl_round!(Vec4<T>, [x, y, z, w]);
impl_round!(Quaternion<T>, [x, y, z, w]);
impl_round!(Mat2<T>, [a11, a12, a21, a22]);
impl_round!(Mat3<T>, [a11, a12, a13, a21, a22, a23, a31, a32, a33]);
impl_round!(Mat4<T>, [a11, a12, a13, a14, a21, a22, a23, a24, a31, a32, a33, a34, a41, a42, a43, a44]);
