
//! A replacement for the default OpenGL matrix stack which is deprecated in newer versions

use cable_math::{Vec3, Mat4};

const STACK_SIZE: usize = 32;

/// A matrix stack containing a single projection matrix and a stack of
/// modelview matrices
pub struct MatrixStack {
    modelview_stack: [Mat4<f32>; STACK_SIZE],
    modelview_pointer: usize,

    projection: Mat4<f32>
}

impl MatrixStack {
    pub fn new() -> MatrixStack {
        MatrixStack {
            modelview_stack: [Mat4::identity(); STACK_SIZE],
            modelview_pointer: 0,
            projection: Mat4::identity(),
        }
    }

    /// Sets the projection matrix to a orthographic projection with the
    /// given parameters
    pub fn ortho(&mut self, left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) {
        self.projection = Mat4::ortho(left, right, bottom, top, near, far);
    }

    /// Pushes one frame onto the modeview stack
    pub fn push(&mut self) {
        if self.modelview_pointer >= STACK_SIZE - 1 {
            panic!("Stack overflow in MatrixStack::push(&mut self)");
        }

        let old_top = self.modelview_stack[self.modelview_pointer];
        self.modelview_pointer += 1;
        self.modelview_stack[self.modelview_pointer] = old_top.clone();
    }

    /// Pops one frame of the modeview stack
    pub fn pop(&mut self) {
        if self.modelview_pointer <= 0 {
            panic!("Stack underflow in MatrixStack::pop(&mut self)");
        }

        self.modelview_pointer -= 1;
    }

    /// Sets the top of the modelview stack to a identity matrix
    pub fn identity(&mut self) {
        self.modelview_stack[self.modelview_pointer] = Mat4::identity();
    }

    /// Applies the given translation to the peek of the modelview stack
    pub fn translate(&mut self, translation: Vec3<f32>) {
        self.modelview_stack[self.modelview_pointer] *= Mat4::translation(translation)
    }

    /// Returns the top of the modelview stack
    pub fn peek(&self) -> Mat4<f32> {
        self.modelview_stack[self.modelview_pointer]
    }

    /// Returns the projection matrix
    pub fn projection(&self) -> Mat4<f32> {
        self.projection
    }

    /// Returns the model-view-projection matrix
    pub fn mvp(&self) -> Mat4<f32> {
        self.projection * self.peek()
    }
}

