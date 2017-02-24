
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
    fn push_private(&mut self) {
        if self.modelview_pointer >= STACK_SIZE - 1 {
            panic!("Stack overflow in MatrixStack::push(&mut self)");
        }

        let old_top = self.modelview_stack[self.modelview_pointer];
        self.modelview_pointer += 1;
        self.modelview_stack[self.modelview_pointer] = old_top.clone();
    }

    /// Pops one frame of the modeview stack
    fn pop_private(&mut self) {
        if self.modelview_pointer <= 0 {
            panic!("Stack underflow in MatrixStack::pop(&mut self)");
        }
        self.modelview_pointer -= 1;
    }

    /// Pushes a frame onto the matrix stack, executes the given action and pops the frame
    /// back off again. All matrix transforms that are executed within the action will be
    /// reset after it returns. This allows for temporary transformations without side effects.
    ///
    /// Note that only the modelview matrix is affected by this, and modifications to the 
    /// projection matrix will persist even after this operation.
    ///
    /// By wrapping the code in a closure we can guarantee that there will never be unbalanced
    /// push-pops.
    ///
    /// # Example
    /// ```
    /// # extern crate gondola;
    /// extern crate cable_math;
    ///
    /// # fn main() {
    /// use gondola::matrix_stack::MatrixStack;
    /// use cable_math::Vec3;
    ///
    /// let mut matrix_stack = MatrixStack::new();
    ///
    /// matrix_stack.push(|matrix_stack| {
    ///     matrix_stack.translate(Vec3::new(2.0, 5.0, 3.0));
    ///
    ///     matrix_stack.push(|matrix_stack| {
    ///         // Nested pushing works just fine
    ///     });
    /// });
    /// # }
    /// // All translations that happened in the above block are reset here
    /// ```
    pub fn push<F>(&mut self, action: F) where F: Fn(&mut Self) {
        self.push_private();
        action(self);
        self.pop_private();
    }

    /// Sets the top of the modelview stack to a identity matrix
    pub fn identity(&mut self) {
        self.modelview_stack[self.modelview_pointer] = Mat4::identity();
    }

    /// Applies the given translation to the top of the modelview stack
    pub fn translate(&mut self, translation: Vec3<f32>) {
        self.modelview_stack[self.modelview_pointer] *= Mat4::translation(translation)
    }

    /// Applies the given scaling to the top of the modelview stack
    pub fn scale(&mut self, scale: Vec3<f32>) {
        self.modelview_stack[self.modelview_pointer] *= Mat4::scaling(scale);
    }

    /// Applies a rotation of `angle` radians around the x-axis to the top of the modelview stack
    pub fn rotate_x(&mut self, angle: f32) {
        self.modelview_stack[self.modelview_pointer] *= Mat4::rotation_x(angle);
    }
    /// Applies a rotation of `angle` radians around the y-axis to the top of the modelview stack
    pub fn rotate_y(&mut self, angle: f32) {
        self.modelview_stack[self.modelview_pointer] *= Mat4::rotation_y(angle);
    }
    /// Applies a rotation of `angle` radians around the z-axis to the top of the modelview stack
    pub fn rotate_z(&mut self, angle: f32) {
        self.modelview_stack[self.modelview_pointer] *= Mat4::rotation_z(angle);
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

