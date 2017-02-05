
use gl;
use gl::types::*;
use std;

const DEFAULT_SIZE: usize = 100;
pub struct GraphicsBuffer {
    buffer: GLuint,
    target: BufferTarget,
    allocated: usize,
    size: usize,
    data_type: DataType,
}

impl GraphicsBuffer {
    /// Initializes a new, empty, buffer
    pub fn new(target: BufferTarget, usage: BufferUsage) -> GraphicsBuffer {
        let mut buffer = 0;

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target.get_gl_enum(), buffer);
            gl::BufferData(target.get_gl_enum(), DEFAULT_SIZE as GLsizeiptr, std::ptr::null(), usage.get_gl_enum());
        }

        GraphicsBuffer {
            buffer: buffer,
            target: target,
            allocated: DEFAULT_SIZE,
            size: 0,
            data_type: DataType::Byte,
        }
    }

    /// Stores the given vector in a new buffer. This assumes usage to be BufferUsage::StaticDraw
    pub fn from_floats(target: BufferTarget, data: Vec<f32>) -> GraphicsBuffer {
        let mut buffer = 0;
        let size = data.len() * std::mem::size_of::<f32>(); // We assume f32 to be equal to GLfloat, which it is

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target.get_gl_enum(), buffer);
            gl::BufferData(
                target.get_gl_enum(),
                size as GLsizeiptr,
                std::mem::transmute(&data[0]),
                BufferUsage::StaticDraw.get_gl_enum()
            );
        }

        GraphicsBuffer {
            buffer: buffer,
            target: target,
            allocated: size,
            size: size,
            data_type: DataType::Float
        }
    }

    /// The number of bytes that are stored in GPU memory
    pub fn len(&self) -> usize {
        self.size
    }

    /// The number of bytes that are internally allocated in GPU memory
    pub fn allocated(&self) -> usize {
        self.allocated
    }

    /// The type of data that is stored in the buffer
    pub fn data_type(&self) -> DataType {
        self.data_type
    }

    /// Binds this buffer to the target specified in the constructor
    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.target.get_gl_enum(), self.buffer);
        }
    }
}

impl Drop for GraphicsBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.buffer);
        }
    }
}

/// Represents different GL buffer usage hints. Note that these are hints,
/// and drivers will not necesarily respect these.
///
/// The first part of the name indicates how frequently the data will be used,
/// the specond part indicates how it will be used:
///
/// * Static - Data is set once and used often 
/// * Dynamic - Data is set frequently and used frequently
/// * Stream - Data is set once and used at most a few times
///
/// * Draw - Data will be set by the application and read by the GPU
/// * Read - Data is set by the GPU and read by the application
/// * Copy - Data is set and read by the GPU
#[derive(Copy, Clone)]
pub enum BufferUsage {
    StaticDraw, DynamicDraw, StreamDraw,
    StaticRead, DynamicRead, StreamRead,
    StaticCopy, DynamicCopy, StreamCopy,
}

impl BufferUsage {
    pub fn get_gl_enum(&self) -> GLenum {
        match *self {
            BufferUsage::StaticDraw  => gl::STATIC_DRAW,
            BufferUsage::DynamicDraw => gl::DYNAMIC_DRAW,
            BufferUsage::StreamDraw  => gl::STREAM_DRAW,
            BufferUsage::StaticRead  => gl::STATIC_READ,
            BufferUsage::DynamicRead => gl::DYNAMIC_READ,
            BufferUsage::StreamRead  => gl::STREAM_READ,
            BufferUsage::StaticCopy  => gl::STATIC_COPY,
            BufferUsage::DynamicCopy => gl::DYNAMIC_COPY,
            BufferUsage::StreamCopy  => gl::STREAM_COPY,
        }
    }
}

/// Reperesents a target to which a buffer can be bound
#[derive(Copy, Clone)]
pub enum BufferTarget {
    ArrayBuffer,
    ElementArrayBuffer,
    PixelPackBuffer,
    PixelUnpackBuffer,
    TransformFeedbackBuffer,
    UniformBuffer,
    TextureBuffer,
    CopyReadBuffer,
    CopyWriteBuffer,
    DrawIndirectBuffer,
    AtomicCounterBuffer,
    DispatchIndirectBuffer,
}

impl BufferTarget {
    pub fn get_gl_enum(&self) -> GLenum {
        match *self {
            BufferTarget::ArrayBuffer             => gl::ARRAY_BUFFER,
            BufferTarget::ElementArrayBuffer      => gl::ELEMENT_ARRAY_BUFFER,
            BufferTarget::PixelPackBuffer         => gl::PIXEL_PACK_BUFFER,
            BufferTarget::PixelUnpackBuffer       => gl::PIXEL_UNPACK_BUFFER,
            BufferTarget::TransformFeedbackBuffer => gl::TRANSFORM_FEEDBACK_BUFFER,
            BufferTarget::UniformBuffer           => gl::UNIFORM_BUFFER,
            BufferTarget::TextureBuffer           => gl::TEXTURE_BUFFER,
            BufferTarget::CopyReadBuffer          => gl::COPY_READ_BUFFER,
            BufferTarget::CopyWriteBuffer         => gl::COPY_WRITE_BUFFER,
            BufferTarget::DrawIndirectBuffer      => gl::DRAW_INDIRECT_BUFFER,
            BufferTarget::AtomicCounterBuffer     => gl::ATOMIC_COUNTER_BUFFER,
            BufferTarget::DispatchIndirectBuffer  => gl::DISPATCH_INDIRECT_BUFFER,
        }
    }
}

/// Represents different types of data which may be stored in a buffer
#[derive(Copy, Clone)]
pub enum DataType {
    Float, Int, Byte, //There are more valid types in gl, but I rarely use those
}

impl DataType {
    pub fn get_gl_enum(&self) -> GLenum {
        match *self {
            DataType::Float => gl::FLOAT,
            DataType::Int   => gl::INT,
            DataType::Byte  => gl::BYTE, 
        }
    }

    pub fn size(&self) -> usize {
        match *self {
            DataType::Float => std::mem::size_of::<f32>(),
            DataType::Int   => std::mem::size_of::<i32>(),
            DataType::Byte  => std::mem::size_of::<i8>(),
        }
    }
}

