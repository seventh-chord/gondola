
use nalgebra::{Orthographic3, Matrix4, Eye};

const STACK_SIZE: usize = 32;

pub struct MatrixStack {
    model_stack: [Matrix4<f32>; STACK_SIZE],
    view_stack: [Matrix4<f32>; STACK_SIZE],
    model_stack_pointer: usize,
    view_stack_pointer: usize,
    projection: Matrix4<f32>
}

impl MatrixStack {
    pub fn new() -> MatrixStack {
        MatrixStack {
            model_stack: [Matrix4::new_identity(4); STACK_SIZE],
            view_stack: [Matrix4::new_identity(4); STACK_SIZE],
            model_stack_pointer: 0,
            view_stack_pointer: 0,
            projection: Matrix4::new_identity(4),
        }
    }

    /// Sets the projection matrix to a orthographic projection with the
    /// given parameters
    pub fn ortho(&mut self, left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) {
        self.projection = Orthographic3::new(left, right, bottom, top, near, far).to_matrix();
    }

    /// Pushes one frame onto the modeview stack
    pub fn push(&mut self) {
        if self.model_stack_pointer >= STACK_SIZE - 1 {
            panic!("Stack overflow in MatrixStack::push(&mut self)");
        }

        let old_top = self.model_stack[self.model_stack_pointer];
        self.model_stack_pointer += 1;
        self.model_stack[self.model_stack_pointer] = old_top.clone();
    }

    /// Pops one frame of the modeview stack
    pub fn pop(&mut self) {
        if self.model_stack_pointer <= 0 {
            panic!("Stack underflow in MatrixStack::pop(&mut self)");
        }

        self.model_stack_pointer -= 1;
    }

    pub fn peek(&self) -> Matrix4<f32> {
        self.model_stack[self.model_stack_pointer]
    }
}
