
use num::*;
use mat::Mat4;
use vec::Vec3;
use std::ops::*;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Quaternion<T: Num + Float + Copy> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub z: T,
}

impl<T: Num + Float + Copy> Quaternion<T> {
    /// Creates a new identity quaternion
    pub fn new() -> Quaternion<T> {
        Quaternion {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
            w: T::one(),
        }
    }

    /// Creates a quaternion representing a counterclockwise rotation of `angle` radians around the 
    /// given axis. This function normalizes the axis, but if the axis is `(0, 0, 0)` the quaternion
    /// will have its `x`, `y` and `z` fields set to 0.
    pub fn rotation(angle: T, axis: Vec3<T>) -> Quaternion<T> {
        let axis = axis.normalize();
        let angle = angle / (T::one() + T::one());
        let (sin, cos) = angle.sin_cos();
        Quaternion {
            x: axis.x*sin,
            y: axis.y*sin,
            z: axis.z*sin,
            w: cos,
        }
    }

    /// Calculates the length of this quaternion, raised to the power of two. Note that this is
    /// cheaper than computing the actual length.
    pub fn len_sqr(&self) -> T {
        self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w
    }

    /// Calculates the length of this quaternion.
    pub fn len(&self) -> T {
        (self.x*self.x + self.y*self.y + self.z*self.z + self.w*self.w).sqrt()
    }

    /// Normalizes this quaternion, returning a new quaternion with length 1.
    pub fn normalize(&self) -> Quaternion<T> {
        *self / self.len()
    }
}

impl<T: Num + Float + Copy> Mul for Quaternion<T> {
    type Output = Self; 
    fn mul(self, other: Quaternion<T>) -> Self {
        Quaternion {
            x: self.w*other.x + self.x*other.w + self.y*other.z - self.z*other.y,
            y: self.w*other.y + self.y*other.w + self.z*other.x - self.x*other.z,
            z: self.w*other.z + self.z*other.w + self.x*other.y - self.y*other.x,
            w: self.w*other.w - self.x*other.x - self.y*other.y - self.z*other.z,
        }
    }
}
impl<T: Num + Float + Copy> MulAssign for Quaternion<T> {
    fn mul_assign(&mut self, other: Quaternion<T>) {
        let x = self.w*other.x + self.x*other.w + self.y*other.z - self.z*other.y;
        let y = self.w*other.y + self.y*other.w + self.z*other.x - self.x*other.z;
        let z = self.w*other.z + self.z*other.w + self.x*other.y - self.y*other.x;
        let w = self.w*other.w - self.x*other.x - self.y*other.y - self.z*other.z;
        self.x = x;
        self.y = y;
        self.z = z;
        self.w = w;
    }
}
impl<T: Num + Float + Copy> Mul<T> for Quaternion<T> {
    type Output = Self; 
    fn mul(self, scalar: T) -> Self {
        Quaternion {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}
impl<T: Num + Float + Copy> MulAssign<T> for Quaternion<T> {
    fn mul_assign(&mut self, scalar: T) {
        self.x = self.x * scalar;
        self.y = self.y * scalar;
        self.z = self.z * scalar;
        self.w = self.w * scalar;
    }
}
impl<T: Num + Float + Copy> Div<T> for Quaternion<T> {
    type Output = Self; 
    fn div(self, scalar: T) -> Self {
        Quaternion {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
            w: self.w / scalar,
        }
    }
}
impl<T: Num + Float + Copy> DivAssign<T> for Quaternion<T> {
    fn div_assign(&mut self, scalar: T) {
        self.x = self.x / scalar;
        self.y = self.y / scalar;
        self.z = self.z / scalar;
        self.w = self.w / scalar;
    }
}

impl<T: Num + Float + Copy> From<Quaternion<T>> for Mat4<T> {
    fn from(quat: Quaternion<T>) -> Mat4<T> {
        Mat4::from_quaternion(quat.x, quat.y, quat.z, quat.w)
    }
}

#[cfg(test)]
mod tests {
    use std::f32;
    use super::*;
    use {Mat4, Vec4};

    #[test]
    fn identity() {
        let identity = Quaternion::<f32>::new();
        assert_eq!(identity, identity*identity);
        assert_eq!(identity, identity*identity*identity);
    }

    #[test]
    fn matrix_rotation() {
        let quat = Quaternion::<f32>::rotation(f32::consts::PI/2.0, Vec3::new(1.0, 0.0, 0.0));
        let mat: Mat4<f32> = quat.into();

        let a = Vec4::new(0.0, 1.0, 0.0, 1.0);
        let b = Vec4::new(0.0, 0.0, 1.0, 1.0);

        let dif = (mat*a - b).len();
        assert!(dif < 0.001);
    }

    #[test]
    fn matrix_rotation_reverse() {
        let mat_a: Mat4<f32> = Quaternion::rotation(f32::consts::PI/2.0, Vec3::new(1.0, 0.0, 0.0)).into();
        let mat_b: Mat4<f32> = Quaternion::rotation(-f32::consts::PI/2.0, Vec3::new(1.0, 0.0, 0.0)).into();

        let a = Vec4::new(0.0, 1.0, 0.0, 1.0);

        let dif = (mat_a*mat_b*a - a).len();
        assert!(dif < 0.001);
    }
}
