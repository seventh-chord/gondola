
//! Tests that rotation functions on different types behave equally 

extern crate cable_math;

use cable_math::{Vec3, Vec4, Mat4, Quaternion};

// Random angles between
const TEST_ANGLES: [f32; 100] = [
    2.7004, 2.512, 2.3236, -1.57, -1.6328, -2.8888, -0.942, 2.1352, 2.7004, 1.6956, 1.8212, 1.3816, 1.4444, 2.9516, 
    -2.6376, -2.512, 2.826, 3.14, -3.14, -0.8164, -2.3864, 1.256, 1.6328, 0.942, -2.198, 0.628, -1.3188, -2.5748, -2.7004, 
    -1.884, -1.57, -2.7004, -1.0676, 3.14, 2.6376, -0.1256, 0.4396, 1.1304, -2.2608, 0.3768, -1.57, 0.1256, 2.8888, 3.14, 
    2.198, 1.3188, 1.7584, 0.6908, -1.57, -1.3188, -2.1352, 3.0772, 0.3768, 1.3816, -1.9468, 1.5072, -1.3188, -0.4396, -1.57, 
    1.6956, -2.1352, -3.0772, -1.57, 2.4492, -2.512, -0.0628, -2.8888, 2.1352, 1.0048, 1.6956, 0.7536, -0.4396, -3.0144, -0.628, 
    -1.6956, -1.0676, 2.826, 0.628, 2.9516, 0.4396, -2.0096, 1.6328, -0.0628, -0.942, 0.942, -2.0096, 0.5652, 1.3816, 1.256, 
    -2.9516, -2.9516, 0.3768, 0.4396, 0.7536, -1.6328, -2.2608, -2.198, -1.0676, 0.942, -2.4492, 
];

fn test_equal(a: Vec3<f32>, b: Vec3<f32>) {
    let diff = (a - b).len();
    assert!(diff < 0.001, "Expected a = {} and b = {} to be equal.", a, b);
}

#[test]
fn quaternion_vector_rotation_coherency() {
    let mut a = Vec3::new(1.0, 0.0, 0.0);
    let mut b = a;

    for angle in TEST_ANGLES.iter() {
        let angle = *angle;

        a = a.rotate_x(angle);
        b = Quaternion::rotation(angle, Vec3::new(1.0, 0.0, 0.0)) * b;
        test_equal(a, b);

        a = a.rotate_y(angle);
        b = Quaternion::rotation(angle, Vec3::new(0.0, 1.0, 0.0)) * b;
        test_equal(a, b);

        a = a.rotate_z(angle);
        b = Quaternion::rotation(angle, Vec3::new(0.0, 0.0, 1.0)) * b;
        test_equal(a, b);
    }
}

#[test]
fn matrix_vector_rotation() {
    let mut a = Vec3::new(1.0, 0.0, 0.0);
    let mut b = Vec4::from3(a, 1.0);

    for angle in TEST_ANGLES.iter() {
        let angle = *angle;

        a = a.rotate_x(angle);
        b = Mat4::rotation_x(angle) * b;
        test_equal(a, b.xyz());

        a = a.rotate_y(angle);
        b = Mat4::rotation_y(angle) * b;
        test_equal(a, b.xyz());

        a = a.rotate_z(angle);
        b = Mat4::rotation_z(angle) * b;
        test_equal(a, b.xyz());
    }
}

#[test]
fn quat_to_matrix_vector_rotation() {
    let mut a = Vec3::new(1.0, 0.0, 0.0);
    let mut b = Vec4::from3(a, 1.0);

    for angle in TEST_ANGLES.iter() {
        let angle = *angle;

        a = a.rotate_x(angle);
        let quat = Quaternion::rotation(angle, Vec3::new(1.0, 0.0, 0.0));
        b = Mat4::from(quat) * b;
        test_equal(a, b.xyz());

        a = a.rotate_y(angle);
        let quat = Quaternion::rotation(angle, Vec3::new(0.0, 1.0, 0.0));
        b = Mat4::from(quat) * b;
        test_equal(a, b.xyz());

        a = a.rotate_z(angle);
        let quat = Quaternion::rotation(angle, Vec3::new(0.0, 0.0, 1.0));
        b = Mat4::from(quat) * b;
        test_equal(a, b.xyz());
    }
}
