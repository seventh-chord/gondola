
use {Vec2, Vec3, Vec4, Mat2, Mat3, Mat4, Quaternion};

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
    ($ty:ty, [$($field:ident),*]) => {
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
