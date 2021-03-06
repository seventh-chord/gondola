
use std::{mem, ptr};
use std::ops::Range;
use std::marker::PhantomData;

use gl;
use gl::types::*;

use super::*;

/// A GPU buffer which holds a set of primitives (floats, bytes or integers). These primitives
/// can be rendered using a [`VertexArray`](struct.VertexArray.html).
pub struct PrimitiveBuffer<T: VertexData> {
    phantom: PhantomData<T>,

    pub(super) buffer: GLuint,
    target: BufferTarget,
    usage: BufferUsage,

    primitive_count: usize, // Used space, in units of T
    allocated: usize, // Allocated space, in units of T
}

/// Contains information on how to render a group of primitive buffers. In most cases simply using
/// a [`VertexBuffer`], which combines information about rendering with data, is more adequate.
///
/// [`VertexBuffer`]: struct.VertexBuffer.html
pub struct VertexArray {
    array: GLuint,
    index_type: Option<GLenum>,
}

impl VertexArray {
    pub fn new() -> VertexArray {
        let mut array = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut array);
        }

        VertexArray {
            array: array,
            index_type: None,
        }
    }

    pub fn bind(&self) {
        unsafe { gl::BindVertexArray(self.array) };
    }

    /// Adds a buffer from which this vertex array will pull data when drawing
    ///
    /// `index` specifies the vertex attribute index to which this data source will be bound. This
    /// is used from glsl through `layout(location = index) in ...;`
    ///
    /// `size` specifies the number of primitives per vertex to use. e.g.: `3` means pull sets of
    /// three vertices from the source and present them as a `vec3` in glsl.
    ///
    /// `stride` gives the distance from the start of the first vertex to the start of the next
    /// vertex. e.g.: If you have a buffer with the contents `[x, y, z, r, g, b, x, y, z, r, g,
    /// b]`, you could use a stride of `6` to indicate that you have to advance 6 primitives to get
    /// from one color to the next color.
    ///
    /// `offset` is the number of primitives at the beginning of the source to skip.
    ///
    /// *NB* Both `stride` and `offset` are in units of `T::Primitive`!
    ///
    ///
    /// `divisor` specifis whether to use one value per vertex (`0`), one value for each instance
    /// (`1`), one value for every two instances (`2`), etc.
    ///
    /// NB (Morten, 04.11.17) Currently, this does not work for integer primitives! In that case,
    /// we need to call `glVertexAttribIPointer` instead!
    pub fn add_data_source<T>(
        &mut self,
        source: &PrimitiveBuffer<T>,
        index:   u32, 
        size:    u32, 
        stride:  u32,
        offset:  u32,
        divisor: u32,
    ) 
      where T: VertexData
    {
        assert!(!T::Primitive::IS_INTEGER); // See end of doc comment

        source.bind();
        unsafe { 
            gl::BindVertexArray(self.array);

            gl::EnableVertexAttribArray(index);

            let primitive_bytes = mem::size_of::<T::Primitive>() as u32;

            gl::VertexAttribPointer(
                index, size as GLint,
                T::Primitive::GL_ENUM, false as GLboolean,
                (stride * primitive_bytes) as GLsizei, 
                (offset * primitive_bytes) as *const GLvoid
            );

            gl::VertexAttribDivisor(index, divisor);
        }
    }

    /// Registers the given primitive buffer to be used as a index buffer (also referred to as
    /// element buffer) for this vertex array.  After this call, calls to [`draw_elements`] are 
    /// safe. Note that `T` must have a primitive type ([`VertexData::Primitive`]) which is 
    /// indexable ([`GlIndex`]). This includes all basic unsigned integers.
    ///
    /// [`GlIndex`]:               trait.GlIndex.html
    /// [`VertexData::Primitive`]: trait.VertexData.html#associatedtype.Primitive
    /// [`draw_elements`]:         #method.draw_elements
    pub fn set_index_buffer<T>(&mut self, buffer: &PrimitiveBuffer<T>) 
      where T: VertexData,
            T::Primitive: GlIndex,
    {
        unsafe {
            gl::BindVertexArray(self.array);
            buffer.bind();
        } 

        self.index_type = Some(T::Primitive::GL_ENUM);
    }

    /// Draws the given type of primitive with the data in the graphics buffers bound to this vertex 
    /// array. If you want to specify indices when drawing use [`draw_elements`] instead.
    ///
    /// [`draw_elements`]: #method.draw_elements
    pub fn draw(&self, mode: PrimitiveMode, range: Range<usize>) {
        unsafe {
            gl::BindVertexArray(self.array);
            gl::DrawArrays(
                mode as GLenum,
                range.start as GLint,
                (range.end - range.start) as GLsizei
            );
        }
    }

    pub fn draw_instanced(&self, mode: PrimitiveMode, range: Range<usize>, instances: usize) {
        unsafe {
            gl::BindVertexArray(self.array);
            gl::DrawArraysInstanced(
                mode as GLenum,
                range.start as GLint,
                (range.end - range.start) as GLsizei,
                instances as GLsizei
            );
        }
    }

    /// Draws the given type of primitives with the data in graphics buffers bound to this vertex
    /// array, in the order specified by the set index buffer (See [`set_index_buffer`]). If you
    /// have not set a index buffer this function will panic at runtime. You might want to use
    /// [`draw`] instead.
    ///
    /// [`set_index_buffer`]: #method.set_index_buffer
    /// [`draw`]: #method.draw
    pub fn draw_elements(&self, mode: PrimitiveMode, count: usize) {
        if let Some(index_type) = self.index_type {
            unsafe {
                gl::BindVertexArray(self.array);
                gl::DrawElements(mode as GLenum, count as GLsizei, index_type, ptr::null());
            }
        } else {
            panic!("VertexArray::draw_elements called without a valid index buffer set!");
        }
    }
}

impl<T: VertexData> PrimitiveBuffer<T> {
    /// Initializes a new, empty, buffer
    pub fn new(target: BufferTarget, usage: BufferUsage) -> PrimitiveBuffer<T> {
        PrimitiveBuffer {
            phantom: PhantomData,

            buffer: 0,
            target, usage,
            allocated: 0,
            primitive_count: 0,
        }
    }

    /// Initializes a new, empty, buffer with capacity for the given number of elements of type `T`.
    pub fn with_capacity(target: BufferTarget, usage: BufferUsage, initial_capacity: usize) -> PrimitiveBuffer<T> {
        let mut buffer = 0;
        let bytes = initial_capacity * mem::size_of::<T>();

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(
                target as GLenum,
                bytes as GLsizeiptr,
                ptr::null(),
                usage as GLenum
            );
        }

        PrimitiveBuffer {
            phantom: PhantomData,

            buffer,
            target,
            usage,
            allocated: initial_capacity,
            primitive_count: 0,
        }
    }

    /// Stores the given data in a new buffer. The buffer will have its usage set to `BufferUsage::StaticDraw`
    pub fn with_data(target: BufferTarget, data: &[T]) -> PrimitiveBuffer<T> {
        if data.is_empty() {
            return PrimitiveBuffer::new(target, BufferUsage::StaticDraw);
        }

        let mut buffer = 0;
        let bytes = data.len() * mem::size_of::<T>();

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(
                target as GLenum,
                bytes as GLsizeiptr,
                mem::transmute(&data[0]),
                BufferUsage::StaticDraw as GLenum
            );
        }

        PrimitiveBuffer {
            phantom: PhantomData,

            buffer: buffer,
            target: target,
            usage: BufferUsage::StaticDraw,
            allocated: data.len(),
            primitive_count: data.len(),
        }
    }
    
    /// Puts the given data at the start of this buffer, replacing any vertices
    /// which where previously in that location. This resizes the underlying buffer
    /// if more space is needed to store the new data.
    pub fn put_at_start(&mut self, data: &[T]) {
        self.put(0, data);
    }
    /// Puts the given data at the end of this buffer, behind any data which is
    /// already in it. This resizes the underlying buffer if more space is needed
    /// to store the new data.
    pub fn put_at_end(&mut self, data: &[T]) {
        let end = self.primitive_count;
        self.put(end, data);
    }
    /// Puts the given data at the given index in this buffer, overwriting any
    /// vertices which where previously in that location. This resizes the underlying
    /// buffer if more space is needed to store the new data.
    ///
    /// The index should be in units of the size of `T`. Thus, for a `PrimitiveBuffer<f32>`, a
    /// index of `2` will start writing data at the eight byte.
    pub fn put(&mut self, index: usize, data: &[T]) {
        if data.is_empty() {
            return;
        }

        let start = index;
        let end = index + data.len();

        let full_override = start == 0 && end >= self.primitive_count;
        self.ensure_allocated(end, !full_override);

        if end > self.primitive_count {
            self.primitive_count = end;
        }

        unsafe {
            gl::BindBuffer(self.target as GLenum, self.buffer);
            gl::BufferSubData(
                self.target as GLenum,
                (start * mem::size_of::<T>()) as GLintptr,
                (data.len() * mem::size_of::<T>()) as GLsizeiptr,
                mem::transmute(&data[0])
            );
        }
    }
    
    /// Sets the number of vertices that can be stored in this buffer without reallocating memory.
    /// If the buffer already has capacity for the given number of vertices no space will be
    /// allocated.
    /// If `retain_old_data` is `false` this will zero out all data if it decides to reallocate
    pub fn ensure_allocated(&mut self, new_size: usize, retain_old_data: bool) {
        // Only reallocate if necessary
        if new_size > self.allocated {
            let bytes = new_size * mem::size_of::<T>();

            let mut new_vbo = 0;

            unsafe {
                gl::GenBuffers(1, &mut new_vbo);
                gl::BindBuffer(BufferTarget::Array as GLenum, new_vbo);
                gl::BufferData(BufferTarget::Array as GLenum, bytes as GLsizeiptr, ptr::null(), self.usage as GLenum);

                // Copy old data
                if retain_old_data && self.buffer != 0 {
                    gl::BindBuffer(BufferTarget::CopyRead as GLenum, self.buffer);
                    gl::CopyBufferSubData(
                        BufferTarget::CopyRead as GLenum,
                        BufferTarget::Array as GLenum,
                        0, 0,
                        (self.primitive_count*mem::size_of::<T>()) as GLsizeiptr
                    );
                    gl::DeleteBuffers(1, &mut self.buffer);
                }
            }

            self.buffer = new_vbo;
            self.allocated = new_size
        }
    }

    /// Empties this buffer by setting its length to 0.
    pub fn clear(&mut self) {
        self.primitive_count = 0;
    }

    /// The number of `T`s stored in this buffer.
    pub fn len(&self) -> usize {
        self.primitive_count
    }
    
    /// The number of whole bytes allocated, divided by the number of bytes per `T`
    pub fn capacity(&self) -> usize {
        self.allocated
    }
    
    /// The number of primitives stored in this buffer. Note that a single `T` may contain
    /// multiple primitives. For example, a `Vec3<f32>` contains three primitives.
    pub fn primitives(&self) -> usize {
        self.primitive_count * T::primitives()
    }

    /// The number of bytes stored in this buffer.
    pub fn bytes(&self) -> usize {
        self.primitive_count * mem::size_of::<T>()
    }

    /// The number of bytes that are internally allocated in GPU memory.
    pub fn bytes_allocated(&self) -> usize {
        self.allocated * mem::size_of::<T>()
    }

    /// Binds this buffer to the target specified in the constructor.
    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.target as GLenum, self.buffer);
        }
    }

    /// Calls `glBindBufferBase` for this buffer, with the given index. This is used
    /// in conjunctions with e.g. uniform buffers.
    pub fn bind_base(&self, index: usize) {
        unsafe {
            gl::BindBufferBase(self.target as GLenum, index as GLuint, self.buffer);
        }
    }
}

impl<T: VertexData> Drop for PrimitiveBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.buffer);
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.array);
        }
    }
}

