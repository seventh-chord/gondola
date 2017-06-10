
//! Wrappers for unsafe OpenGL calls

use gl;
use gl::types::*;

use cable_math::Vec2;

use {Color, Region};

/// Sets the OpenGL viewport
///
/// Because `gl::Scissor` takes integers as parameters the given regions coordinates will be cast
/// before being used. 
pub fn viewport(region: Region) {
    unsafe {
        gl::Viewport(
            region.min.x as GLint, region.min.y as GLint,
            region.max.x as GLint, region.max.y as GLint,
        );
    }
}

/// Enables/disables the OpenGL scissor test.  The given region is in screen space, that is, in the
/// same coordinate system as [`viewport`]. Anything drawn outside this region will be discarded.
///
/// `gl::Scissor` takes integers as parameters the given regions coordinates will be cast
/// before being used. 
///
/// Because OpenGL requires a scissor region to be specified from the bottom left, but everything
/// else in this library operates from the top left this function needs to know the window size.
///
/// [`viewport`]: fn.viewport.html
pub fn set_scissor(region: Option<Region>, win_size: Vec2<f32>) {
    unsafe {
        if let Some(region) = region {
            gl::Enable(gl::SCISSOR_TEST);

            gl::Scissor(
                region.min.x as GLint,
                (win_size.y - region.min.y - region.height()) as GLint,
                region.width() as GLint,
                region.height() as GLint,
            )
        } else {
            gl::Disable(gl::SCISSOR_TEST);
        }
    }
} 

/// Prints all OpenGL errors.
pub fn print_errors() {
    unsafe {
        while let Some(error) = get_error_message(gl::GetError()) {
            println!("OpenGL error: {}", error);
        }
    }
}
fn get_error_message(error: GLenum) -> Option<String> {
    let value = match error {
        gl::INVALID_VALUE                   => "Invalid value",
        gl::INVALID_ENUM                    => "Invalid enum",
        gl::INVALID_OPERATION               => "Invalid operation",
        gl::INVALID_FRAMEBUFFER_OPERATION   => "Invalid framebuffer operation",
        gl::OUT_OF_MEMORY                   => "Out of memory",

        gl::NO_ERROR                        => return None,
        _                                   => return Some(format!("Invalid error code: {:x}", error)),
    };
    Some(String::from(value))
}

/// Sets which side of a face to treat as the front face and which side of a face to cull. If
/// `None` is passed this disables culling.
///
/// For simplicity, you can simply call `graphics::set_culling(Some(Default::default()))`,
/// which will set the winding order to counter-clockwise and cull-face to the back face.
pub fn set_culling(mode: Option<(WindingOrder, FaceSide)>) {
    unsafe { match mode {
        Some((winding_order, face_side)) => {
            gl::Enable(gl::CULL_FACE);
            match winding_order {
                WindingOrder::Clockwise => gl::FrontFace(gl::CW),
                WindingOrder::CounterClockwise => gl::FrontFace(gl::CCW),
            }
            match face_side {
                FaceSide::Front => gl::CullFace(gl::FRONT),
                FaceSide::Back => gl::CullFace(gl::BACK),
            }
        },
        None => {
            gl::Disable(gl::CULL_FACE);
        },
    } }
}

#[derive(Debug, Copy, Clone)]
pub enum WindingOrder {
    Clockwise, CounterClockwise,
} 
#[derive(Debug, Copy, Clone)]
pub enum FaceSide {
    Front, Back
}

impl Default for WindingOrder { 
    /// The default winding order is counter-clockwise.
    fn default() -> WindingOrder { WindingOrder::CounterClockwise } 
}
impl Default for FaceSide { 
    /// The default face side is back, as this is the face you want to cull by default.
    fn default() -> FaceSide { FaceSide::Back } 
}

/// Enables/disables rasterization. If disabled, only the vertex shader will be run. This is
/// useful when you are only interested in transform feedback. Keep in mind that rasterization
/// has to be re-enabled before rendering, otherwise nothing will be shown.
pub fn set_rasterization(discard: bool) {
    if discard {
        unsafe { gl::Enable(gl::RASTERIZER_DISCARD) };
    } else {
        unsafe { gl::Disable(gl::RASTERIZER_DISCARD) };
    }
} 

/// Clears the currently bound framebuffer to the given color.
pub fn clear(color: Option<Color>, depth: bool, stencil: bool) {
    unsafe {
        if let Some(color) = color {
            gl::ClearColor(color.r, color.g, color.b, color.a);
        }
        let mut mask = 0;
        if color.is_some() { mask |= gl::COLOR_BUFFER_BIT }
        if depth           { mask |= gl::DEPTH_BUFFER_BIT }
        if stencil         { mask |= gl::STENCIL_BUFFER_BIT }
        gl::Clear(mask);
    }
}

/// Toggles depth testing. This only has an effect if the currently bound framebuffer
/// has a depthbuffer (The backbuffer always has a depthbuffer).
pub fn set_depth_testing(enabled: bool) {
    unsafe {
        if enabled {
            gl::Enable(gl::DEPTH_TEST);
        } else {
            gl::Disable(gl::DEPTH_TEST);
        }
    }
}

/// Sets the function used to check if a fragment passes the depth test. The initial value is
/// `Less`. See [`DepthFunction`] for more info.
///
/// [`DepthFunction`]: enum.DepthFunction.html
pub fn set_depth_function(depth_function: DepthFunction) {
    unsafe {
        gl::DepthFunc(depth_function as GLenum);
    }
}

#[repr(u32)] // GLenum is u32
#[derive(Copy, Clone, Debug)]
pub enum DepthFunction {
    /// The depth test never passes.
    Never           = gl::NEVER,
    /// The depth test always passes.
    Always         = gl::ALWAYS,
    /// Only passes if the new fragment is at exactly the same depth as the old fragment.
    Equal           = gl::EQUAL,
    /// Only passes if the new fragment is at a different depth from the old fragment.
    NotEqual        = gl::NOTEQUAL,

    /// Only passes if the new fragment is closer than the old fragment.
    Less            = gl::LESS,
    /// Only passes if the new fragment is closer than or at the same depth as the old fragment.
    LessOrEqual     = gl::LEQUAL,

    /// Only passes if the new fragment is further away than the old fragment.
    Greater         = gl::GREATER,
    /// Only passes if the new fragment is further away or at the same depth as the old fragment.
    GreaterOrEqual  = gl::GEQUAL,
}

/// If passed `Some` enables the given blend settings. If passed `None` disables
/// blending.
pub fn set_blending(blending: Option<BlendSettings>) {
    unsafe {
        if let Some(ref settings) = blending {
            gl::Enable(gl::BLEND);

            gl::BlendFuncSeparate(
                settings.src_color as GLenum,
                settings.dst_color as GLenum,
                settings.src_alpha as GLenum,
                settings.dst_alpha as GLenum
            );
            gl::BlendEquation(settings.function as GLenum);
        } else {
            gl::Disable(gl::BLEND);
        }
    }
}

/// Settings used to define OpenGL blend state. You should create a pair of settings
/// for every operation which uses blending, and apply those settings before rendering.
/// Blending can be enabled either through
/// [`enable()`](struct.BlendSettings.html#method.enable)
/// or
/// [`graphics::set_blending(Some(my_settings))`](fn.set_blending.html)
/// and can be disabled wtih
/// [`graphics::set_blending(None)`](fn.set_blending.html).
///
/// Note that this struct implements `Default`, so default blend settings can be retrieved
/// with `BlendSettings::default()`.
#[derive(Debug, Clone, Copy)]
pub struct BlendSettings {
    pub src_color:  BlendFactor,
    pub src_alpha:  BlendFactor,
    pub dst_color:  BlendFactor,
    pub dst_alpha:  BlendFactor,
    pub function:   BlendFunction,
}

impl Default for BlendSettings {
    fn default() -> BlendSettings {
        BlendSettings {
            src_color:  BlendFactor::SrcAlpha,
            dst_color:  BlendFactor::OneMinusSrcAlpha,
            src_alpha:  BlendFactor::One,
            dst_alpha:  BlendFactor::Zero,
            function:   BlendFunction::Add,
        }
    }
}

#[repr(u32)] // GLenum is u32
#[derive(Copy, Clone, Debug)]
pub enum BlendFactor {
    Zero                    = gl::ZERO,
    One                     = gl::ONE,
    SrcColor                = gl::SRC_COLOR,
    OneMinusSrcColor        = gl::ONE_MINUS_SRC_COLOR,
    DstColor                = gl::DST_COLOR,
    OneMinusDstColor        = gl::ONE_MINUS_DST_COLOR,
    SrcAlpha                = gl::SRC_ALPHA,
    OneMinusSrcAlpha        = gl::ONE_MINUS_SRC_ALPHA,
    DstAlpha                = gl::DST_ALPHA,
    OneMinusDstAlpha        = gl::ONE_MINUS_DST_ALPHA,
    ConstantColor           = gl::CONSTANT_COLOR,
    OneMinusConstantColor   = gl::ONE_MINUS_CONSTANT_COLOR,
    ConstantAlpha           = gl::CONSTANT_ALPHA,
    OneMinusConstantAlpha   = gl::ONE_MINUS_CONSTANT_ALPHA,
}

#[repr(u32)] // GLenum is u32
#[derive(Copy, Clone, Debug)]
pub enum BlendFunction {
    /// `Src + Dst`
    Add             = gl::FUNC_ADD,
    /// `Src - Dst`
    Subtract        = gl::FUNC_SUBTRACT,
    /// `Dst - Src`
    ReverseSubtract = gl::FUNC_REVERSE_SUBTRACT,
    /// `min(Dst, Src)`
    Min             = gl::MIN,
    /// `max(Dst, Src)`
    Max             = gl::MAX,
}
