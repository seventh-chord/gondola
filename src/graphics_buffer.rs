
use gl;
use gl::types::*;
use std;

const DEFAULT_SIZE: usize = 100;
pub struct GraphicsBuffer {
    buffer: GLuint,
    target: BufferTarget,
    usage: BufferUsage,
    allocated: usize,
    primitives: usize,
    data_type: DataType,
}

impl GraphicsBuffer {
    /// Initializes a new, empty, buffer
    pub fn new(target: BufferTarget, usage: BufferUsage) -> GraphicsBuffer {
        let mut buffer = 0;

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(target as GLenum, DEFAULT_SIZE as GLsizeiptr, std::ptr::null(), usage as GLenum);
        }

        GraphicsBuffer {
            buffer: buffer,
            target: target,
            usage: usage,
            allocated: DEFAULT_SIZE,
            primitives: 0,
            data_type: DataType::Byte,
        }
    }

    /// Stores the given vector in a new buffer. This assumes usage to be BufferUsage::StaticDraw
    pub fn from_floats(target: BufferTarget, data: Vec<f32>) -> GraphicsBuffer {
        let mut buffer = 0;
        let byte_count = data.len() * DataType::Float.size(); // We assume f32 to be equal to GLfloat, which it is

        unsafe {
            gl::GenBuffers(1, &mut buffer);
            gl::BindBuffer(target as GLenum, buffer);
            gl::BufferData(
                target as GLenum,
                byte_count as GLsizeiptr,
                std::mem::transmute(&data[0]),
                BufferUsage::StaticDraw as GLenum
            );
        }

        GraphicsBuffer {
            buffer: buffer,
            target: target,
            usage: BufferUsage::StaticDraw,
            allocated: byte_count,
            primitives: data.len(),
            data_type: DataType::Float
        }
    }

    /// Stores the given vector into this buffer
    pub fn put_floats(&mut self, data: Vec<f32>) {
        self.data_type = DataType::Float;
        self.primitives = data.len();
        let byte_count = data.len() * DataType::Float.size(); 

        unsafe {
            gl::BindBuffer(self.target as GLenum, self.buffer);

            //Resize if necesarry
            if self.allocated < byte_count {
                gl::BufferData(
                    self.target as GLenum,
                    byte_count as GLsizeiptr,
                    std::mem::transmute(&data[0]),
                    self.usage as GLenum
                );
            } else {
                gl::BufferSubData(
                    self.target as GLenum,
                    0 as GLintptr, byte_count as GLsizeiptr,
                    std::mem::transmute(&data[0])
                );
            }
        }
    }

    /// The number of primitives that are stored in GPU memory. Note that this is
    /// *different* from the number of bytes stored.
    pub fn len(&self) -> usize {
        self.primitives
    }

    /// The number of bytes that are internally allocated in GPU memory
    pub fn bytes_allocated(&self) -> usize {
        self.allocated
    }

    /// The type of data that is stored in the buffer
    pub fn data_type(&self) -> DataType {
        self.data_type
    }

    /// Binds this buffer to the target specified in the constructor
    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.target as GLenum, self.buffer);
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
    StaticDraw  = gl::STATIC_DRAW as isize,
    DynamicDraw = gl::DYNAMIC_DRAW as isize,
    StreamDraw  = gl::STREAM_DRAW as isize,
    StaticRead  = gl::STATIC_READ as isize,
    DynamicRead = gl::DYNAMIC_READ as isize,
    StreamRead  = gl::STREAM_READ as isize,
    StaticCopy  = gl::STATIC_COPY as isize,
    DynamicCopy = gl::DYNAMIC_COPY as isize,
    StreamCopy  = gl::STREAM_COPY as isize,
}

/// Reperesents a target to which a buffer can be bound
#[derive(Copy, Clone)]
pub enum BufferTarget {
    ArrayBuffer             = gl::ARRAY_BUFFER as isize,
    ElementArrayBuffer      = gl::ELEMENT_ARRAY_BUFFER as isize,
    PixelPackBuffer         = gl::PIXEL_PACK_BUFFER as isize,
    PixelUnpackBuffer       = gl::PIXEL_UNPACK_BUFFER as isize,
    TransformFeedbackBuffer = gl::TRANSFORM_FEEDBACK_BUFFER as isize,
    UniformBuffer           = gl::UNIFORM_BUFFER as isize,
    TextureBuffer           = gl::TEXTURE_BUFFER as isize,
    CopyReadBuffer          = gl::COPY_READ_BUFFER as isize,
    CopyWriteBuffer         = gl::COPY_WRITE_BUFFER as isize,
    DrawIndirectBuffer      = gl::DRAW_INDIRECT_BUFFER as isize,
    AtomicCounterBuffer     = gl::ATOMIC_COUNTER_BUFFER as isize,
    DispatchIndirectBuffer  = gl::DISPATCH_INDIRECT_BUFFER as isize,
}

/// Represents different types of data which may be stored in a buffer
#[derive(Copy, Clone)]
pub enum DataType {
    Float = gl::FLOAT as isize,
    Int   = gl::INT as isize,
    Byte  = gl::BYTE as isize,
    //There are more valid types in gl, but I rarely use those
}

impl DataType {
    pub fn size(&self) -> usize {
        match *self {
            DataType::Float => std::mem::size_of::<f32>(),
            DataType::Int   => std::mem::size_of::<i32>(),
            DataType::Byte  => std::mem::size_of::<i8>(),
        }
    }
}

