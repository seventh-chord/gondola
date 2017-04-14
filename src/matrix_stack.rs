
//! A replacement for the default OpenGL matrix stack which is deprecated in newer versions

use gl;
use gl::types::*;
use cable_math::{Vec3, Mat4};
use buffer::*;

const STACK_SIZE: usize = 32;

/// A matrix stack containing a single projection matrix and a stack of
/// modelview matrices
pub struct MatrixStack {
    model_stack: [Mat4<f32>; STACK_SIZE],
    model_pointer: usize,
    view_stack: [Mat4<f32>; STACK_SIZE],
    view_pointer: usize,
    projection: Mat4<f32>,

    uniform_buffer_index: GLuint,
    uniform_buffer: PrimitiveBuffer<Mat4<f32>>,
}

impl MatrixStack {
    pub fn new() -> MatrixStack {
        let uniform_buffer = PrimitiveBuffer::<Mat4<f32>>::new(BufferTarget::Uniform, BufferUsage::DynamicDraw);

        MatrixStack {
            model_stack: [Mat4::identity(); STACK_SIZE],
            model_pointer: 0,
            view_stack: [Mat4::identity(); STACK_SIZE],
            view_pointer: 0,
            projection: Mat4::identity(),

            uniform_buffer_index: get_uniform_binding_index(),
            uniform_buffer: uniform_buffer,
        }
    }

    /// Sets the projection matrix to a orthographic projection with the given parameters
    pub fn ortho(&mut self, left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) {
        self.projection = Mat4::ortho(left, right, bottom, top, near, far);
    }

    /// Sets the projection matrix to a perspective projection with the given parameters.
    /// `fov` is the vertical field of view and should be given in degrees.
    pub fn perspective(&mut self, fov: f32, aspect: f32, near: f32, far: f32) {
        self.projection = Mat4::perspective(fov, aspect, near, far);
    }

    /// Pushes one frame onto the model stack
    fn push_private(&mut self) {
        if self.model_pointer >= STACK_SIZE - 1 {
            panic!("Stack overflow in MatrixStack::push(&mut self)");
        }

        let old_top = self.model_stack[self.model_pointer];
        self.model_pointer += 1;
        self.model_stack[self.model_pointer] = old_top.clone();
    }

    /// Pops one frame of the model stack
    fn pop_private(&mut self) {
        if self.model_pointer <= 0 {
            panic!("Stack underflow in MatrixStack::pop(&mut self)");
        }
        self.model_pointer -= 1;
    }

    /// Pushes one frame onto the modeview stack
    fn view_push_private(&mut self) {
        if self.view_pointer >= STACK_SIZE - 1 {
            panic!("Stack overflow in MatrixStack::push(&mut self)");
        }

        let old_top = self.view_stack[self.view_pointer];
        self.view_pointer += 1;
        self.view_stack[self.view_pointer] = old_top.clone();
    }

    /// Pops one frame of the modeview stack
    fn view_pop_private(&mut self) {
        if self.view_pointer <= 0 {
            panic!("Stack underflow in MatrixStack::pop(&mut self)");
        }
        self.view_pointer -= 1;
    }

    /// Pushes a frame onto the matrix stack, executes the given action and pops the frame
    /// back off again. All matrix transforms that are executed within the action will be
    /// reset after it returns. This allows for temporary transformations without side effects.
    ///
    /// Note that only the model matrix is affected by this, and modifications to the 
    /// projection matrix will persist even after this operation.
    ///
    /// By wrapping the code in a closure we can guarantee that there will never be unbalanced
    /// push-pops.
    ///
    /// # Example
    /// ```rust,no_run We need a gl context to create a matrix stack
    /// # #![allow(unused_variables)]
    ///
    /// # extern crate gondola;
    /// extern crate cable_math;
    ///
    /// # fn main() {
    /// use gondola::MatrixStack;
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
    pub fn push<F>(&mut self, mut action: F) where F: FnMut(&mut Self) {
        self.push_private();
        action(self);
        self.pop_private();
    }

    /// Equal to `push`, but modifies the view matrix rather than the model matrix. See [`push`][1]
    /// for more info.
    ///
    /// [1]: struct.MatrixStack.html#fn.push.html
    pub fn push_view<F>(&mut self, mut action: F) where F: FnMut(&mut Self) {
        self.view_push_private();
        action(self);
        self.view_pop_private();
    }

    /// Sets the top of the model and view stacks to a identity matrix
    pub fn identity(&mut self) {
        self.view_stack[self.view_pointer] = Mat4::identity();
        self.model_stack[self.model_pointer] = Mat4::identity();
    }

    /// Applies the given translation to the top of the model stack
    pub fn translate(&mut self, translation: Vec3<f32>) {
        self.model_stack[self.model_pointer] *= Mat4::translation(translation)
    }

    /// Applies the given scaling to the top of the model stack
    pub fn scale(&mut self, scale: Vec3<f32>) {
        self.model_stack[self.model_pointer] *= Mat4::scaling(scale);
    }

    /// Applies a rotation of `angle` radians around the x-axis to the top of the model stack
    pub fn rotate_x(&mut self, angle: f32) {
        self.model_stack[self.model_pointer] *= Mat4::rotation_x(angle);
    }
    /// Applies a rotation of `angle` radians around the y-axis to the top of the model stack
    pub fn rotate_y(&mut self, angle: f32) {
        self.model_stack[self.model_pointer] *= Mat4::rotation_y(angle);
    }
    /// Applies a rotation of `angle` radians around the z-axis to the top of the model stack
    pub fn rotate_z(&mut self, angle: f32) {
        self.model_stack[self.model_pointer] *= Mat4::rotation_z(angle);
    }

    /// Returns the top of the model stack
    pub fn peek(&self) -> Mat4<f32> {
        self.model_stack[self.model_pointer]
    }

    /// Applies the given translation to the top of the view stack
    pub fn translate_view(&mut self, translation: Vec3<f32>) {
        self.view_stack[self.view_pointer] *= Mat4::translation(translation)
    }

    /// Applies the given scaling to the top of the view stack
    pub fn scale_view(&mut self, scale: Vec3<f32>) {
        self.view_stack[self.view_pointer] *= Mat4::scaling(scale);
    }

    /// Applies a rotation of `angle` radians around the x-axis to the top of the view stack
    pub fn rotate_x_view(&mut self, angle: f32) {
        self.view_stack[self.view_pointer] *= Mat4::rotation_x(angle);
    }
    /// Applies a rotation of `angle` radians around the y-axis to the top of the view stack
    pub fn rotate_y_view(&mut self, angle: f32) {
        self.view_stack[self.view_pointer] *= Mat4::rotation_y(angle);
    }
    /// Applies a rotation of `angle` radians around the z-axis to the top of the view stack
    pub fn rotate_z_view(&mut self, angle: f32) {
        self.view_stack[self.view_pointer] *= Mat4::rotation_z(angle);
    }

    /// Returns the top of the view stack
    pub fn peek_view(&self) -> Mat4<f32> {
        self.view_stack[self.view_pointer]
    }

    /// Returns the projection matrix
    pub fn projection(&self) -> Mat4<f32> {
        self.projection
    }

    /// Returns the model-view-projection matrix
    pub fn mvp(&self) -> Mat4<f32> {
        self.projection * self.peek_view() * self.peek()
    }

    /// Writes the model-view-projection matrix, the model matrix and the normal matrix to 
    /// the uniform buffer to which all shaders have access. Note that shaders need to be 
    /// set up in order to have access to this buffer. This is done automatically when 
    /// constructing a shader with the `load_shader!()` macro, or can be done manually by 
    /// calling `bind_to_matrix_storage()` on a `ShaderPrototype` before building a shader 
    /// from it.
    pub fn update_buffer(&mut self) {
        let mvp = self.mvp();
        let model = self.peek();
        let normal = model.inverse().transpose();

        self.uniform_buffer.put_at_start(&[mvp, model, normal]);
        self.uniform_buffer.bind_base(self.uniform_buffer_index);
    }
}

/// Retrives the uniform binding index at which matricies are stored.
/// *This is for internal use only.*
pub fn get_uniform_binding_index() -> GLuint {
    unsafe {
        let mut index = 0;
        gl::GetIntegerv(gl::MAX_UNIFORM_BUFFER_BINDINGS, &mut index);
        (index - 1) as GLuint
    }
}

