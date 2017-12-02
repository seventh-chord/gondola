
use cable_math::Vec2;

use Region;
use input::{KeyState, Input, Gamepad, GamepadButton};
use graphics;

// Since most of the lib is written expecting gl 3.3 we currently don't allow customizing this.
#[derive(Debug, Copy, Clone)]
pub struct GlRequest {
    version: (u32, u32),
    core: bool,
    debug: bool,
    forward_compatible: bool,
}

impl Default for GlRequest {
    fn default() -> GlRequest {
        GlRequest {
            version: (3, 3),
            core: true,
            debug: cfg!(debug_assertions),
            forward_compatible: false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(usize)]
pub enum CursorType {
    Normal,
    Clickable,
    Invisible,
}

const CURSOR_TYPE_COUNT: usize = 3;
const ALL_CURSOR_TYPES: [CursorType; CURSOR_TYPE_COUNT] = [
    CursorType::Normal,
    CursorType::Clickable,
    CursorType::Invisible,
];

/// Because a different `struct Window` is used per platform, all functions are defined on this
/// trait.
///
/// # Example
/// ```rust,no_run
/// use gondola::{Window, WindowCommon};
///
/// let mut window = Window::new("My title");
///
/// while !window.close_requested {
///     // Update and render
///
///     window.swap_buffers();
/// }
/// ```
pub trait WindowCommon: Drop {
    fn new(title: &str) -> Self;
    fn show(&mut self);

    fn poll_events(&mut self, input: &mut Input);
    fn swap_buffers(&mut self);

    fn close_requested(&self) -> bool;
    fn resized(&self) -> bool;
    fn moved(&self) -> bool;
    /// The region covered by the window, in display space. Use `Region::size` to find the size of
    /// the window.
    fn screen_region(&self) -> Region;
    fn focused(&self) -> bool;

    fn change_title(&mut self, title: &str);
    /// Enables/disables vsync, if supported by the graphics driver. In debug mode a warning is
    /// printed when calling this function if changing vsync is not supported. By default, vsync is
    /// disabled.
    fn set_vsync(&mut self, vsync: bool);

    /// Sets the visual apperance of the cursor when it is inside this window
    fn set_cursor(&mut self, cursor: CursorType);
    /// Clips the cursor so it can not leave the given region. The region should be in window
    /// space. That is, the region is relative to the top-left of this windows screen region.
    fn clip_cursor(&mut self, region: Option<Region>);
    /// Constrains the cursor to the center of the screen. This takes precedence over `clip_cursor`
    fn grab_cursor(&mut self, grabbed: bool);
}

#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "linux")]
mod linux {
    extern crate x11_dl;

    use super::*;

    use std::ptr;
    use std::mem;
    use std::str;
    use std::ffi::CString;

    use gl;

    // We access all ffi stuff through `ffi::whatever` instead of through each apis specific
    // bindings. This allows us to easily add custom stuff that is missing in bindings.
    mod ffi {
        pub(super) use super::x11_dl::xlib::*;
        pub(super) use super::x11_dl::glx::*;
        pub(super) use super::x11_dl::glx::arb::*;

        pub const GLX_RGBA_TYPE: i32 = 0x8014; // From /usr/include/GL/glx.h

        #[allow(non_camel_case_types)]
        pub type glXSwapIntervalEXT = extern "system" fn(*mut Display, GLXDrawable, i32);
    }

    pub struct Window {
        xlib: ffi::Xlib,
        glx: ffi::Glx,

        display: *mut ffi::Display,
        window: u64,

        im: ffi::XIM,
        ic: ffi::XIC,

        wm_delete_window: ffi::Atom,
        cursors: [u64; CURSOR_TYPE_COUNT],
        swap_function: ffi::glXSwapIntervalEXT,

        close_requested: bool,
        resized: bool,
        moved: bool,
        cursor_grabbed: bool,
        cursor_clip_region: Option<Region>,
        cursor: CursorType,
        focused: bool,

        screen_region: Region,
    }

    impl WindowCommon for Window {
        fn new(title: &str) -> Window {
            let gl_request = GlRequest::default();

            // Load xlib and glx
            let xlib = match ffi::Xlib::open() {
                Ok(x) => x,
                Err(err) => {
                    panic!("Could not load xlib: {:?}", err);
                },
            };

            let glx = match ffi::Glx::open() {
                Ok(x) => x,
                Err(err) => {
                    panic!("Could not load glx: {:?}", err);
                },
            };

            unsafe { (xlib.XInitThreads)() };
            unsafe { (xlib.XSetErrorHandler)(Some(x_error_callback)) };

            // Create display
            let display = unsafe { 
                let display = (xlib.XOpenDisplay)(ptr::null());

                if display.is_null() {
                    panic!("Could not connect to the X server");
                }

                display
            };

            // Set up OpenGL
            let mut attributes = [
                ffi::GLX_X_RENDERABLE,  1,
                ffi::GLX_DRAWABLE_TYPE, ffi::GLX_WINDOW_BIT,
                ffi::GLX_RENDER_TYPE,   ffi::GLX_RGBA_BIT,
                ffi::GLX_X_VISUAL_TYPE, ffi::GLX_TRUE_COLOR,
                ffi::GLX_RED_SIZE,      8,
                ffi::GLX_GREEN_SIZE,    8,
                ffi::GLX_BLUE_SIZE,     8,
                ffi::GLX_ALPHA_SIZE,    8,
                ffi::GLX_DEPTH_SIZE,    24,
                ffi::GLX_STENCIL_SIZE,  8,
                ffi::GLX_DOUBLEBUFFER,  1,

                0,
            ];

            let default_screen = unsafe { (xlib.XDefaultScreen)(display) };

            let mut count = 0;
            let fb_configs = unsafe { (glx.glXChooseFBConfig)(
                display,
                default_screen,
                attributes.as_mut_ptr(),
                &mut count,
            ) };
            if fb_configs.is_null() {
                panic!("No FB configs");
            }

            let fb_config = unsafe { *fb_configs }; // Just use the first one, whatever
            unsafe { (xlib.XFree)(fb_configs as *mut _) };

            let visual = unsafe { (glx.glXGetVisualFromFBConfig)(display, fb_config) };
            if visual.is_null() {
                panic!("No appropriate visual found");
            }

            // Create window
            let root = unsafe { (xlib.XDefaultRootWindow)(display) };

            let colormap = unsafe { (xlib.XCreateColormap)(display, root, (*visual).visual, 0) };

            let mut win_attributes = ffi::XSetWindowAttributes {
                event_mask: 
                    ffi::ExposureMask |
                    ffi::StructureNotifyMask |
                    ffi::PointerMotionMask |
                    ffi::KeyPressMask | ffi::KeyReleaseMask |
                    ffi::ButtonPressMask | ffi::ButtonReleaseMask |
                    ffi::FocusChangeMask,

                colormap: colormap,

                .. unsafe { mem::zeroed() }
            };

            let center = Vec2::new(500.0, 400.0);
            let size = Vec2::new(1024.0, 576.0);
            let screen_region = Region {
                min: center/2.0 - size/2.0,
                max: center/2.0 + size/2.0,
            };

            let window = unsafe { (xlib.XCreateWindow)(
                display, root,
                screen_region.min.x as i32, screen_region.min.y as i32,
                screen_region.width() as u32, screen_region.height() as u32,
                0, // Border

                (*visual).depth, // Depth
                ffi::InputOutput as _,
                (*visual).visual,

                ffi::CWColormap | ffi::CWEventMask,
                &mut win_attributes,
            ) };

            unsafe { (xlib.XFree)(visual as *mut _); }

            let title = CString::new(title).unwrap();
            unsafe { (xlib.XStoreName)(display, window, title.into_raw()); }

            // Load cursors
            let cursors = unsafe {
                let mut cursors: [u64; CURSOR_TYPE_COUNT] = mem::uninitialized();

                for (i, &ty) in ALL_CURSOR_TYPES.iter().enumerate() {
                    if ty == CursorType::Invisible {
                        let no_data = [0i8; 8*8];
                        let mut black = ffi::XColor { 
                            pixel: 0, red: 0, green: 0, blue: 0, flags: 0, pad: 0 
                        };
                        let bitmap_no_data = (xlib.XCreateBitmapFromData)(
                            display, window, no_data.as_ptr(), 8, 8
                        );

                        cursors[i] = (xlib.XCreatePixmapCursor)(
                            display,
                            bitmap_no_data, bitmap_no_data,
                            &mut black, &mut black, 0, 0
                        );
                    } else {
                        // Stuff is not defined in the x11 crate, and I can't be arsed to create proper
                        // definitions, so I just copy the values here from `/usr/include/X11/cursorfont.h`
                        let cursor = match ty {
                            CursorType::Normal    => 2,
                            CursorType::Clickable => 58, // or 60 for different hand
                            CursorType::Invisible => 0,
                        };

                        cursors[i] = (xlib.XCreateFontCursor)(display, cursor);
                    }
                }

                cursors
            };

            // Finish setting up OpenGL
            // (_context is not used anywhere, hence the underscore)
            let _context = unsafe {
                #[allow(non_camel_case_types)]
                type glXCreateContextAttribsARB = extern "system" fn(
                    *mut ffi::Display,
                    ffi::GLXFBConfig,
                    ffi::GLXContext,
                    i32,
                    *const i32
                ) -> ffi::GLXContext;

                let create_fn = (glx.glXGetProcAddress)(b"glXCreateContextAttribsARB\0".as_ptr());

                let context = if let Some(create_fn) = create_fn {
                    let profile_mask = if gl_request.core {
                        ffi::GLX_CONTEXT_CORE_PROFILE_BIT_ARB
                    } else {
                        ffi::GLX_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB
                    };

                    let mut flags = 0;
                    if gl_request.debug {
                        flags |= ffi::GLX_CONTEXT_DEBUG_BIT_ARB;
                    }
                    if gl_request.forward_compatible {
                        flags |= ffi::GLX_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB;
                    }

                    let context_attributes = [
                        ffi::GLX_CONTEXT_MAJOR_VERSION_ARB, gl_request.version.0 as i32,
                        ffi::GLX_CONTEXT_MINOR_VERSION_ARB, gl_request.version.1 as i32,
                        ffi::GLX_CONTEXT_FLAGS_ARB, flags,
                        ffi::GLX_CONTEXT_PROFILE_MASK_ARB, profile_mask,
                        0,
                    ];

                    let create_fn = mem::transmute::<_, glXCreateContextAttribsARB>(create_fn);

                    create_fn(
                        display, fb_config, 
                        ptr::null_mut(), 1,
                        context_attributes.as_ptr(),
                    )
                } else {
                    println!("Could not use glXCreateContextAttribsARB!");
                    (glx.glXCreateNewContext)(
                        display, fb_config,
                        ffi::GLX_RGBA_TYPE,
                        ptr::null_mut(), 1
                    )
                };

                if context.is_null() {
                    panic!("Could not create GLX context for the given request: {:?}", gl_request);
                }

                (glx.glXMakeCurrent)(display, window, context);
                context
            };

            let mut gl_name_buf = Vec::with_capacity(500);
            gl::load_with(|name| {
                gl_name_buf.clear();
                gl_name_buf.extend_from_slice(name.as_bytes());
                gl_name_buf.push(0);

                unsafe {
                    (glx.glXGetProcAddress)(gl_name_buf.as_ptr()).unwrap() as *const _
                }
            });
            
            unsafe {
                let raw = gl::GetString(gl::VERSION);
                if raw.is_null() {
                    panic!("glGetString(GL_VERSION) returned null!");
                }
    //            let version = CStr::from_ptr(raw as *const _).to_string_lossy();
    //            println!("{}", version);
            }

            // Vsync stuff
            // TODO: This is not completly correct, we should be checking for extensions
            // before retrieving the function. See https://www.khronos.org/opengl/wiki/Swap_Interval
            // for more info.
            let swap_function = unsafe { 
                let function = (glx.glXGetProcAddress)(b"glXSwapIntervalEXT\0".as_ptr());
                if let Some(function) = function {
                    mem::transmute::<_, ffi::glXSwapIntervalEXT>(function)
                } else {
                    panic!(
                        "Could not retrieve glXSwapIntervalEXT."
                    )
                }
            };

            // Disable vsync initially
            swap_function(display, window, 0);

            // Create IM and IC (Input method and context)
            let im = unsafe {
                let im = (xlib.XOpenIM)(display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());

                if im.is_null() {
                    panic!("xlib::XOpenIM failed");
                }
                im
            };

            let ic = unsafe {
                let ic = (xlib.XCreateIC)(
                    im, 
                    b"inputStyle\0".as_ptr() as *const _,
                    ffi::XIMPreeditNothing | ffi::XIMStatusNothing,
                    b"clientWindow\0".as_ptr() as *const _,
                    window,
                    ptr::null::<()>(),
                );

                if ic.is_null() {
                    panic!("xlib::XCreateIC failed");
                }
                ic
            };

            graphics::viewport(screen_region.unpositioned());

            // Listen for close events
            let wm_delete_window = unsafe {
                let mut atom = (xlib.XInternAtom)(
                    display,
                    b"WM_DELETE_WINDOW\0".as_ptr() as *const _,
                    0
                );
                (xlib.XSetWMProtocols)(display, window, &mut atom, 1);
                atom
            };

            Window {
                xlib, glx,
                display,
                window,
                im,
                ic,
                wm_delete_window,
                cursors,
                swap_function,
                screen_region,

                close_requested: false,
                resized: false,
                moved: false,
                cursor_grabbed: false,
                cursor: CursorType::Normal,
                cursor_clip_region: None,
                focused: false,
            }
        }

        fn show(&mut self) {
            unsafe { (self.xlib.XMapWindow)(self.display, self.window); }
        }

        fn poll_events(&mut self, input: &mut Input) {
            input.refresh();

            self.moved = false;
            self.resized = false;
            self.close_requested = false;

            // Handle events
            unsafe { while (self.xlib.XPending)(self.display) > 0 {
                let mut event = mem::zeroed::<ffi::XEvent>();
                (self.xlib.XNextEvent)(self.display, &mut event);
                let ty = event.get_type();

                match ty {
                    ffi::Expose => {
                        // Sent whenever the screen should be redrawn. We can ignore this, since we
                        // continually redraw screen contents anyways.
                    },

                    ffi::FocusIn => {
                        let cursor = self.cursor;
                        self.internal_set_cursor(cursor);

                        if self.cursor_grabbed {
                            self.internal_grab_cursor(true);
                        }

                        self.focused = true;
                        input.window_has_keyboard_focus = self.focused;
                    },

                    ffi::FocusOut => {
                        self.internal_grab_cursor(false);
                        self.internal_set_cursor(CursorType::Normal);

                        self.focused = false;
                        input.window_has_keyboard_focus = self.focused;
                    },

                    ffi::KeyPress | ffi::KeyRelease => {
                        input.received_events_this_frame = true;
                        let mut event: ffi::XKeyEvent = event.into();

                        // Normal key input
                        let scancode = event.keycode;

                        let ref mut state = input.keys[scancode as usize];
                        *state = if ty == ffi::KeyPress {
                            if state.down() {
                                KeyState::PressedRepeat
                            } else {
                                KeyState::Pressed
                            }
                        } else {
                            KeyState::Released
                        };

                        // Typing
                        if ty == ffi::KeyPress {
                            let mut buffer = [0u8; 16];
                            let mut status: ffi::Status = 0;

                            let count = (self.xlib.Xutf8LookupString)(
                                self.ic, &mut event,
                                mem::transmute(buffer.as_mut_ptr()),
                                buffer.len() as _,
                                ptr::null_mut(), &mut status,
                            );

                            if status != ffi::XBufferOverflow {
                                let text = str::from_utf8(&buffer[..count as usize]).unwrap_or("");
                                input.type_buffer.push_str(text);
                            } else {
                                // Try again with a dynamic buffer
                                let mut buffer = vec![0u8; count as usize];
                                let count = (self.xlib.Xutf8LookupString)(
                                    self.ic, &mut event,
                                    mem::transmute(buffer.as_mut_ptr()),
                                    buffer.len() as _,
                                    ptr::null_mut(), &mut status
                                );

                                let text = str::from_utf8(&buffer[..count as usize]).unwrap_or("");
                                input.type_buffer.push_str(text);
                            }
                        }
                    },

                    // Mouse buttons
                    ffi::ButtonPress | ffi::ButtonRelease => {
                        input.received_events_this_frame = true;

                        let event: ffi::XButtonEvent = event.into();

                        let state = if ty == ffi::ButtonPress {
                            KeyState::Pressed
                        } else {
                            KeyState::Released
                        };

                        match event.button {
                            // X11 uses different button indices
                            1 => input.mouse_keys[0] = state,
                            2 => input.mouse_keys[2] = state,
                            3 => input.mouse_keys[1] = state,
                            
                            // Scrolling
                            4 | 5 if state == KeyState::Pressed => {
                                let scroll = if event.button == 4 { 1.0 } else { -1.0 };
                                input.mouse_scroll += scroll;
                            },

                            _ => {},
                        };
                    },

                    // Mouse movement
                    ffi::MotionNotify => {
                        input.received_events_this_frame = true;

                        let event: ffi::XMotionEvent = event.into();

                        let new_pos = Vec2::new(event.x, event.y).as_f32();
                        if new_pos != input.mouse_pos {
                            let delta = new_pos - input.mouse_pos;
                            input.mouse_delta += delta;
                            input.raw_mouse_delta += delta;

                            input.mouse_pos = new_pos;
                        }

                        if self.focused && !self.cursor_grabbed {
                            if let Some(region) = self.cursor_clip_region {
                                let pos = input.mouse_pos;
                                let new_pos = region.clip(pos);

                                if pos != new_pos {
                                    input.mouse_pos = new_pos;

                                    (self.xlib.XWarpPointer)(
                                        self.display, 0, self.window,
                                        0, 0, 0, 0,
                                        new_pos.x as i32, new_pos.y as i32,
                                    );
                                    (self.xlib.XFlush)(self.display);
                                }
                            }
                        }
                    },

                    ffi::MappingNotify => {
                        (self.xlib.XRefreshKeyboardMapping)(event.as_mut());
                    },

                    ffi::ConfigureNotify => {
                        let event: ffi::XConfigureEvent = event.into();

                        let pos = Vec2::new(event.x, event.y).as_f32();
                        let size = Vec2::new(event.width, event.height).as_f32();

                        let new_region = Region {
                            min: pos,
                            max: pos + size,
                        };

                        if new_region.min != self.screen_region.min {
                            self.moved = true;
                        }

                        if new_region.size() != self.screen_region.size() {
                            self.resized = true;
                        }

                        self.screen_region = new_region;
                        graphics::viewport(self.screen_region.unpositioned());
                    },
                    ffi::ReparentNotify => {},
                    ffi::MapNotify => {},

                    ffi::ClientMessage => {
                        let event: ffi::XClientMessageEvent = event.into();

                        if event.data.get_long(0) == self.wm_delete_window as i64 {
                            self.close_requested = true;
                        }
                    },

                    other => {
                        panic!("Unkown X event type: {}", other);
                    },
                }
            } }

            // Constrain cursor if it is grabbed or clipped
            if self.focused {
                if self.cursor_grabbed {
                    let center = self.screen_region.unpositioned().center().as_i32();
                    input.mouse_pos = center.as_f32();

                    unsafe {
                        (self.xlib.XWarpPointer)(
                            self.display, 0, self.window,
                            0, 0, 0, 0,
                            center.x, center.y,
                        );
                        (self.xlib.XFlush)(self.display);
                    }
                } else if let Some(region) = self.cursor_clip_region {
                    let pos = input.mouse_pos;
                    let new_pos = region.clip(pos);

                    if pos != new_pos {
                        input.mouse_pos = new_pos;

                        unsafe {
                            (self.xlib.XWarpPointer)(
                                self.display, 0, self.window,
                                0, 0, 0, 0,
                                new_pos.x as i32, new_pos.y as i32,
                            );
                            (self.xlib.XFlush)(self.display);
                        }
                    }
                }
            }
        }

        fn swap_buffers(&mut self) {
            let ref glx = self.glx;

            unsafe {
                (glx.glXSwapBuffers)(self.display, self.window);
            }
        }

        fn close_requested(&self) -> bool   { self.close_requested }
        fn resized(&self) -> bool           { self.resized }
        fn moved(&self) -> bool             { self.resized }
        fn focused(&self) -> bool           { self.focused }
        fn screen_region(&self) -> Region   { self.screen_region }

        fn change_title(&mut self, title: &str) {
            let title = CString::new(title).unwrap();
            unsafe { (self.xlib.XStoreName)(self.display, self.window, title.into_raw()) };
        }

        fn set_vsync(&mut self, vsync: bool) {
            (self.swap_function)(self.display, self.window, if vsync { 1 } else { 0 });
        }

        fn set_cursor(&mut self, cursor: CursorType) {
            if self.cursor == cursor {
                return;
            }
            self.cursor = cursor;
            self.internal_set_cursor(cursor);
        }

        fn clip_cursor(&mut self, region: Option<Region>) {
            self.cursor_clip_region = region;
        }

        fn grab_cursor(&mut self, grabbed: bool) {
            if self.cursor_grabbed == grabbed {
                return;
            }
            self.cursor_grabbed = grabbed;

            if self.focused {
                self.internal_grab_cursor(grabbed);
            }
        }
    }

    impl Window {
        fn internal_grab_cursor(&mut self, grab: bool) {
            unsafe {
                if grab {
                    (self.xlib.XGrabPointer)(
                        self.display, self.window,
                        ffi::True, 0,
                        ffi::GrabModeAsync,
                        ffi::GrabModeAsync,

                        self.window,
                        0, // This is `None` (I think)
                        ffi::CurrentTime,
                    );
                } else {
                    (self.xlib.XUngrabPointer)(self.display, ffi::CurrentTime);
                }
            }
        }

        fn internal_set_cursor(&mut self, cursor: CursorType) {
            unsafe { (self.xlib.XDefineCursor)(
                self.display, self.window,
                self.cursors[cursor as usize],
            ) };
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            let ref xlib = self.xlib;

            unsafe {
                (xlib.XDestroyIC)(self.ic);
                (xlib.XCloseIM)(self.im);

                (xlib.XDestroyWindow)(self.display, self.window);
                (xlib.XCloseDisplay)(self.display);
            }
        }
    }

    unsafe extern "C" fn x_error_callback(
        _display: *mut ffi::Display,
        event: *mut ffi::XErrorEvent
    ) -> i32
    {
        println!("X error: {}", (*event).error_code);
        0
    }
}

#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    extern crate winapi;
    extern crate user32;
    extern crate kernel32;
    extern crate gdi32;
    extern crate opengl32;
    extern crate xinput;

    use std::ptr;
    use std::mem;
    use std::char;
    use std::sync::mpsc;
    use std::cell::RefCell;
    use std::ffi::CStr;

    use gl;

    // We access all ffi stuff through `ffi::whatever` instead of through each apis specific
    // bindings. This allows us to easily add custom stuff that is missing in bindings.
    mod ffi {
        #![allow(non_camel_case_types)]

        pub(super) use super::winapi::*;
        pub(super) use super::user32::*;
        pub(super) use super::kernel32::*;
        pub(super) use super::gdi32::*;
        pub(super) use super::opengl32::*;
        pub(super) use super::xinput::*;

        // Stuff not defined in winapi
        pub(super) const ERROR_INVALID_VERSION_ARB: u32 = 0x2095;
        pub(super) const ERROR_INVALID_PROFILE_ARB: u32 = 0x2096;

        pub(super) const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
        pub(super) const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;
        pub(super) const WGL_CONTEXT_FLAGS_ARB: i32 = 0x2094;
        pub(super) const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;

        pub(super) const WGL_CONTEXT_DEBUG_BIT_ARB: i32 = 0x0001;
        pub(super) const WGL_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB: i32 = 0x0002;

        pub(super) const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x00000001;
        pub(super) const WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB: i32 = 0x00000002;

        pub(super) type wglCreateContextAttribsARBType = extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC;
        pub(super) type wglGetExtensionsStringARBType = extern "system" fn(HDC) -> *const i8;
        pub(super) type wglSwapIntervalEXTType = extern "system" fn(i32) -> i32;
    }

    pub struct Window {
        raw_event_receiver: mpsc::Receiver<RawEvent>,
        device_context: ffi::HDC,
        gl_context: ffi::HGLRC,
        window: ffi::HWND,
        swap_function: Option<ffi::wglSwapIntervalEXTType>,
        cursors: [ffi::HCURSOR; CURSOR_TYPE_COUNT],

        screen_region: Region,
        close_requested: bool,
        resized: bool,
        moved: bool,
        focused: bool,

        cursor: CursorType,
        cursor_captured: bool, // Cursor is dragging something out of the window, don't loose focus on release
        cursor_grabbed: bool, // Cursor cant leave window
        cursor_clip_region: Option<Region>, // Relative to `screen_region.min`!

        gamepad_states: [InternalGamepadState; 4],
    }

    #[derive(Copy, Clone)]
    struct InternalGamepadState {
        connected: bool,
        last_packet_number: u32,
        xinput_state: ffi::XINPUT_STATE,
    }

    impl Default for InternalGamepadState {
        fn default() -> InternalGamepadState {
            InternalGamepadState {
                connected: false,
                last_packet_number: 0,
                xinput_state: unsafe { mem::zeroed() },
            }
        }
    }


    fn encode_wide(s: &str) -> Vec<u16> {
        let mut data = Vec::with_capacity(s.len() + 1);
        for wchar in s.encode_utf16() {
            data.push(wchar);
        }
        data.push(0);
        data
    }

    fn last_win_error() -> u32 { unsafe { ffi::GetLastError() } }

    #[derive(Debug, Copy, Clone)]
    enum RawEvent {
        MoveOrSize,
        CloseRequest,
        Key(bool, usize),
        Char(u16),
        Scroll(f32),
        MousePos(Vec2<f32>),
        MouseDelta(Vec2<f32>),
        MouseButton(bool, usize),
    }

    thread_local! {
        static MSG_SENDER: RefCell<Option<mpsc::Sender<RawEvent>>> = RefCell::new(None);
    }

    // This is WNDPROC
    unsafe extern "system" 
    fn event_callback(window: ffi::HWND, msg: u32, w: ffi::WPARAM, l: ffi::LPARAM) -> ffi::LRESULT {
        let maybe_event = match msg {
            ffi::WM_SIZE | ffi::WM_MOVE => {
                Some(RawEvent::MoveOrSize)
            },

            ffi::WM_CLOSE => {
                Some(RawEvent::CloseRequest)
            },

            ffi::WM_KEYUP | ffi::WM_KEYDOWN => {
                let down         = msg == ffi::WM_KEYDOWN;
                let scancode     = ((l as usize) >> 16) & 0xff;
                //let prev_down    = ((l >> 30 ) & 1) == 1;
                //let repeat_count = (l as usize) & 0xffff;

                Some(RawEvent::Key(down, scancode))
            },

            ffi::WM_CHAR => {
                Some(RawEvent::Char(w as u16))
            },

            ffi::WM_MOUSEWHEEL => {
                let delta = ffi::GET_WHEEL_DELTA_WPARAM(w) as f32 / ffi::WHEEL_DELTA as f32;
                Some(RawEvent::Scroll(delta))
            },

            ffi::WM_MOUSEMOVE => {
                let x = ffi::GET_X_LPARAM(l);
                let y = ffi::GET_Y_LPARAM(l);
                let pos = Vec2::new(x, y).as_f32();
                Some(RawEvent::MousePos(pos))
            },

            ffi::WM_INPUT => {
                let mut bytes = [0u8; 48];
                let mut size = bytes.len() as u32;
                assert_eq!(mem::size_of::<ffi::RAWINPUT>(), size as usize);

                ffi::GetRawInputData(
                    l as _, ffi::RID_INPUT,
                    bytes.as_mut_ptr() as *mut _, &mut size,
                    mem::size_of::<ffi::RAWINPUTHEADER>() as u32,
                );
                let raw_input = (bytes.as_ptr() as *const ffi::RAWINPUT).as_ref().unwrap();

                if raw_input.header.dwType == ffi::RIM_TYPEMOUSE {
                    let x = raw_input.mouse.lLastX;
                    let y = raw_input.mouse.lLastY;
                    let delta = Vec2::new(x, y).as_f32();

                    Some(RawEvent::MouseDelta(delta))
                } else {
                    None
                }
            },

            ffi::WM_LBUTTONDOWN => Some(RawEvent::MouseButton(true, 0)),
            ffi::WM_LBUTTONUP   => Some(RawEvent::MouseButton(false, 0)),
            ffi::WM_MBUTTONDOWN => Some(RawEvent::MouseButton(true, 2)),
            ffi::WM_MBUTTONUP   => Some(RawEvent::MouseButton(false, 2)),
            ffi::WM_RBUTTONDOWN => Some(RawEvent::MouseButton(true, 1)),
            ffi::WM_RBUTTONUP   => Some(RawEvent::MouseButton(false, 1)),

            _ => return ffi::DefWindowProcW(window, msg, w, l), // Maybe we don't need this
        };

        if let Some(event) = maybe_event {
            MSG_SENDER.with(|sender| {
                if let Some(ref sender) = *sender.borrow() {
                    sender.send(event).unwrap();
                } else {
                    panic!("`event_callback` called from unkown thread");
                }
            });
        }

        return 0;
    }

    impl WindowCommon for Window {
        fn new(title: &str) -> Window {
            let gl_request = GlRequest::default();

            let instance = unsafe { ffi::GetModuleHandleW(ptr::null()) };

            let class_name = encode_wide("My windows class is great");
            let window_name = encode_wide(title);

            let window_class = ffi::WNDCLASSW {
                style:          ffi::CS_OWNDC,
                lpfnWndProc:    Some(event_callback),
                hInstance:      instance,
                lpszClassName:  class_name.as_ptr(),

                //            hIcon:          HICON, // Less so

                .. unsafe { mem::zeroed() }
            };

            let window_class_atom = unsafe { ffi::RegisterClassW(&window_class) };
            if window_class_atom == 0 {
                panic!("Failed to register window class");
            }

            let (raw_event_sender, raw_event_receiver) = mpsc::channel();

            MSG_SENDER.with(|sender| {
                let mut sender = sender.borrow_mut();
                if sender.is_some() {
                    panic!("Multiple windows on a single thread are not supported on windows atm");
                }

                *sender = Some(raw_event_sender);
            });

            // Load cursors
            let cursors = unsafe {
                let mut cursors = [ptr::null_mut(); CURSOR_TYPE_COUNT];
                for (i, &ty) in ALL_CURSOR_TYPES.iter().enumerate() {
                    let cursor = match ty {
                        CursorType::Normal    => ffi::IDC_ARROW,
                        CursorType::Clickable => ffi::IDC_HAND,
                        CursorType::Invisible => continue,
                    };
                    cursors[i] = ffi::LoadCursorW(ptr::null_mut(), cursor);
                }
                cursors
            };

            // Actually create window 
            let window = unsafe { ffi::CreateWindowExW(
                // Extended style
                0, 

                class_name.as_ptr(),
                window_name.as_ptr(),

                ffi::WS_OVERLAPPEDWINDOW,

                ffi::CW_USEDEFAULT, ffi::CW_USEDEFAULT,
                ffi::CW_USEDEFAULT, ffi::CW_USEDEFAULT,

                ptr::null_mut(), // Parent
                ptr::null_mut(), // Menu
                instance,
                ptr::null_mut(), // lParam
            ) };
            if window.is_null() {
                panic!("Failed to create window");
            } 

            let region = unsafe {
                let mut rect = new_rect();
                if ffi::GetWindowRect(window, &mut rect) == 0 {
                    panic!("GetWindowRect failed: {}", last_win_error());
                }

                Region {
                    min: Vec2::new(rect.left, rect.top).as_f32(),
                    max: Vec2::new(rect.right, rect.bottom).as_f32(),
                }
            };

            let device_context = unsafe { ffi::GetDC(window) };

            // Set up raw input
            let raw_mouse_device = ffi::RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage:     0x02,
                dwFlags:     ffi::RIDEV_INPUTSINK,
                hwndTarget:  window,
            };
            unsafe { ffi::RegisterRawInputDevices(
                &raw_mouse_device,
                1, mem::size_of::<ffi::RAWINPUTDEVICE>() as u32,
            ) };

            // Choose a pixel format
            let mut pixel_format_descriptor = ffi::PIXELFORMATDESCRIPTOR {
                nSize: mem::size_of::<ffi::PIXELFORMATDESCRIPTOR>() as u16,
                nVersion: 1,
                dwFlags: ffi::PFD_DRAW_TO_WINDOW | ffi::PFD_SUPPORT_OPENGL | ffi::PFD_DOUBLEBUFFER,
                iPixelType: ffi::PFD_TYPE_RGBA,
                cColorBits: 24,
                cAlphaBits: 8,
                iLayerType: ffi::PFD_MAIN_PLANE,

                .. unsafe { mem::zeroed() }
            };

            unsafe {
                let i = ffi::ChoosePixelFormat(device_context, &mut pixel_format_descriptor);
                let result = ffi::SetPixelFormat(device_context, i, &mut pixel_format_descriptor);

                if result == ffi::FALSE {
                    panic!("Failed to set pixel format");
                }
            };

            // We have to load opengl32 to get the proc address for old gl functions (e.g GetString)
            let library_name = b"opengl32.dll\0";
            let gl32_lib = unsafe { ffi::LoadLibraryA(library_name.as_ptr() as *const i8) };
            if gl32_lib.is_null() {
                panic!("Could not load opengl32.dll: {}", last_win_error());
            }

            // Set up opengl context
            let legacy_gl_context = unsafe {
                let c = ffi::wglCreateContext(device_context);
                ffi::wglMakeCurrent(device_context, c);
                c
            };

            let mut gl_name_buf = Vec::with_capacity(500);
            let mut get_proc_address = |name: &str| { 
                gl_name_buf.clear();
                gl_name_buf.extend_from_slice(name.as_bytes());
                gl_name_buf.push(0);

                unsafe {
                    let address = ffi::wglGetProcAddress(gl_name_buf.as_ptr() as *const _);

                    // Acording to the khronos guide, -1, 0, 1, 2 and 3 indicate an error
                    let invalid =
                        address == ((-1isize) as *const _) || address == (0 as *const _) ||
                        address == (1 as *const _) || address == (2 as *const _) || address == (3 as *const _);

                    if invalid {
                        // This is needed for some pre gl 3 functions
                        kernel32::GetProcAddress(gl32_lib, gl_name_buf.as_ptr() as *const _)
                    } else {
                        address
                    }
                }
            }; 

            #[allow(non_snake_case)]
            let wglGetExtensionsStringARB = unsafe {
                let p = get_proc_address("wglGetExtensionsStringARB");
                if p.is_null() {
                    panic!("WGL_ARB_extensions_string is not supported. Can not create a gl context");
                }
                mem::transmute::<_, ffi::wglGetExtensionsStringARBType>(p)
            };

            let extensions = unsafe {
                // This gives us a space separated list of supported extenensions
                let raw = wglGetExtensionsStringARB(device_context);
                let string = CStr::from_ptr(raw).to_string_lossy();
                string.split_whitespace().map(str::to_owned).collect::<Vec<_>>()
            };

            let has_extension = |name: &str| {
                for extension in extensions.iter() {
                    if extension == name {
                        return true;
                    }
                }
                false
            };

            let gl_context = if gl_request.version.0 < 3 {
                legacy_gl_context

                    // Set up modern OpenGL
            } else {
                let required_extensions = [
                    "WGL_ARB_create_context",
                    "WGL_ARB_create_context_profile",
                ];
                for name in required_extensions.iter() {
                    if !has_extension(name) {
                        panic!("{} is not supported. Can not create a gl 3+ context", name);
                    }
                }

                #[allow(non_snake_case)]
                let wglCreateContextAttribsARB = unsafe {
                    let p = get_proc_address("wglCreateContextAttribsARB");
                    if p.is_null() {
                        panic!(
                            "wglCreateContextAttribsARB is not present, although the required \
                            extensions are supported. Your drivers/the spec suck"
                            );
                    }
                    mem::transmute::<_, ffi::wglCreateContextAttribsARBType>(p)
                };

                let mut flags = 0;
                if gl_request.debug {
                    flags |= ffi::WGL_CONTEXT_DEBUG_BIT_ARB;
                }
                if gl_request.forward_compatible {
                    flags |= ffi::WGL_CONTEXT_FORWARD_COMPATIBLE_BIT_ARB;
                }

                let profile_mask = if gl_request.core {
                    ffi::WGL_CONTEXT_CORE_PROFILE_BIT_ARB
                } else {
                    ffi::WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB
                };

                let context_attributes = [
                    ffi::WGL_CONTEXT_MAJOR_VERSION_ARB, gl_request.version.0 as i32,
                    ffi::WGL_CONTEXT_MINOR_VERSION_ARB, gl_request.version.1 as i32,
                    ffi::WGL_CONTEXT_FLAGS_ARB, flags,
                    ffi::WGL_CONTEXT_PROFILE_MASK_ARB, profile_mask,
                    0,
                ];

                let gl_context = wglCreateContextAttribsARB(
                    device_context,
                    ptr::null_mut(),
                    context_attributes.as_ptr()
                    );

                if gl_context.is_null() {
                    let last_error = last_win_error();
                    match last_error {
                        ffi::ERROR_INVALID_VERSION_ARB => panic!(
                            "Could not create GL context. Invalid version: ({}.{} {})",
                            gl_request.version.0, gl_request.version.1,
                            if gl_request.core { "core" } else { "compat" },
                            ),
                        ffi::ERROR_INVALID_PROFILE_ARB => panic!(
                            "Could not create GL context. Invalid profile: ({}.{} {})",
                            gl_request.version.0, gl_request.version.1,
                            if gl_request.core { "core" } else { "compat" },
                            ),
                        _ => panic!(
                            "Could not create GL context. Unkown error: {}",
                            last_error,
                            ),
                    };
                }

                // Replace the legacy context with the new and improved context
                unsafe {
                    ffi::wglDeleteContext(legacy_gl_context);
                    ffi::wglMakeCurrent(device_context, gl_context);
                }

                gl_context
            };

            let swap_function = if has_extension("WGL_EXT_swap_control") {
                Some(unsafe {
                    let p = get_proc_address("wglSwapIntervalEXT");
                    if p.is_null() {
                        panic!(
                            "wglSwapIntervalEXTis not present, although the required \
                            extensions are supported. Your drivers/the specification suck"
                        );
                    }
                    mem::transmute::<_, ffi::wglSwapIntervalEXTType>(p)
                })
            } else {
                None
            };

            gl::load_with(get_proc_address);

            unsafe {
                let raw = gl::GetString(gl::VERSION);
                if raw.is_null() {
                    panic!("glGetString(GL_VERSION) returned null!");
                }
                //            let version = CStr::from_ptr(raw as *const _).to_string_lossy();
                //            println!("{}", version);
            }

            graphics::viewport(region.unpositioned());

            Window {
                raw_event_receiver,
                device_context,
                gl_context,
                window,
                swap_function,
                cursors,

                screen_region: region,
                close_requested: false,
                resized: false,
                moved: false,
                focused: false,

                cursor: CursorType::Normal,
                cursor_captured: false,
                cursor_grabbed: false,
                cursor_clip_region: None,

                gamepad_states: [InternalGamepadState::default(); 4],
            }
        } 

        fn show(&mut self) {
            unsafe { ffi::ShowWindow(self.window, ffi::SW_SHOW) };
        }

        fn poll_events(&mut self, input: &mut Input) {
            let focused = unsafe { ffi::GetFocus() == self.window };
            let focus_changed = self.focused != focused;
            self.focused = focused;
            input.window_has_keyboard_focus = self.focused;

            // Receive events from windows, dispatch them to `event_callback` and let them get sent
            // back through `raw_event_receiver`.
            let mut msg = unsafe { mem::uninitialized::<ffi::MSG>() };
            loop {
                let result = unsafe { ffi::PeekMessageW(
                    &mut msg, self.window, 
                    0, 0,
                    ffi::PM_REMOVE,
                )};

                if result > 0 {
                    unsafe {
                        ffi::TranslateMessage(&mut msg);
                        ffi::DispatchMessageW(&mut msg);
                    }
                } else {
                    break;
                }
            }

            input.refresh();

            self.moved = false;
            self.resized = false;
            self.close_requested = false;

            for raw_event in self.raw_event_receiver.try_iter() {
                use self::RawEvent::*;
                match raw_event {
                    MoveOrSize => {
                        let new_region = unsafe { 
                            let mut rect = new_rect();
                            ffi::GetClientRect(self.window, &mut rect);

                            let mut min = ffi::POINT { x: rect.left,  y: rect.top };
                            let mut max = ffi::POINT { x: rect.right, y: rect.bottom };
                            ffi::ClientToScreen(self.window, &mut min);
                            ffi::ClientToScreen(self.window, &mut max);

                            let min = Vec2::new(min.x, min.y).as_f32();
                            let max = Vec2::new(max.x, max.y).as_f32();

                            Region { min, max }
                        };

                        if new_region.min != self.screen_region.min {
                            self.moved = true;
                        }

                        if new_region.size() != self.screen_region.size() {
                            self.resized = true;
                        }

                        self.screen_region = new_region;
                        graphics::viewport(self.screen_region.unpositioned());

                        self.update_cursor_clip();
                    },

                    CloseRequest => {
                        self.close_requested = true;
                    },

                    Key(pressed, code) => {
                        input.received_events_this_frame = true;

                        let ref mut state = input.keys[code];
                        *state = if pressed {
                            if state.down() {
                                KeyState::PressedRepeat
                            } else {
                                KeyState::Pressed
                            }
                        } else {
                            KeyState::Released
                        };
                    },

                    Char(wchar) => {
                        input.received_events_this_frame = true;

                        for result in char::decode_utf16([wchar].iter().cloned()) {
                            match result {
                                Ok(c) => input.type_buffer.push(c),
                                Err(_) => println!("WM_CHAR with invalid code: {}", wchar),
                            }
                        }
                    },

                    Scroll(delta) => {
                        input.received_events_this_frame = true;
                        input.mouse_scroll += delta;
                    },

                    MousePos(new_pos) => {
                        if new_pos != input.mouse_pos {
                            input.received_events_this_frame = true;

                            input.mouse_delta += new_pos - input.mouse_pos;
                            input.mouse_pos = new_pos;
                        }
                    },

                    MouseDelta(delta) => {
                        if delta != Vec2::ZERO {
                            input.received_events_this_frame = true;
                            input.raw_mouse_delta += delta;
                        }
                    },

                    MouseButton(down, code) => {
                        input.received_events_this_frame = true;

                        let state = if down { KeyState::Pressed } else { KeyState::Released };
                        input.mouse_keys[code] = state;

                        let mut any_down = false;
                        for state in input.mouse_keys.iter() {
                            if state.down() {
                                any_down = true;
                                break;
                            }
                        }

                        // As long as any mouse buttons are down we want to capture the mouse. This
                        // allows draging stuff around to work even when the mouse temporarily
                        // leaves the window.
                        let cursor_captured = any_down;
                        if cursor_captured != self.cursor_captured {
                            self.cursor_captured = cursor_captured;
                            if self.cursor_captured {
                                unsafe { ffi::SetCapture(self.window) };
                            } else {
                                unsafe { ffi::ReleaseCapture() };
                            }
                        }
                    },
                }
            }

            if focus_changed {
                self.update_cursor_clip();
            }

            if self.focused && self.cursor_grabbed {
                let global_center = self.screen_region.center().as_i32();
                let relative_center = self.screen_region.unpositioned().center().as_i32();
                input.mouse_pos = relative_center.as_f32();
                unsafe { ffi::SetCursorPos(global_center.x, global_center.y) };
            }

            // Change cursor graphic
            if self.focused && self.cursor_in_window() {
                let cursor = self.cursors[self.cursor as usize];
                unsafe { ffi::SetCursor(cursor) };
            } else if focus_changed {
                let cursor = self.cursors[CursorType::Normal as usize];
                unsafe { ffi::SetCursor(cursor) };
            }
            
            // XInput gamepad mess
            for (index, state) in self.gamepad_states.iter_mut().enumerate() {
                let result = unsafe { ffi::XInputGetState(index as u32, &mut state.xinput_state) };

                // TODO don't retry connecting all the time, as that lags. I think
                // casey talked about this at some point, in one of the pubg streams.
                // It would be a pain in the ass to find though.

                if result == ffi::ERROR_SUCCESS {
                    state.connected = true;
                } else if result == ffi::ERROR_DEVICE_NOT_CONNECTED {
                    state.connected = false;
                } else {
                    println!("Unexpected return from `XInputGetState`: {}", result);
                }

                if !state.connected {
                    continue;
                }

                if state.last_packet_number != state.xinput_state.dwPacketNumber {
                    input.received_events_this_frame = true;
                }
                state.last_packet_number = state.xinput_state.dwPacketNumber;

                let ref mut s = state.xinput_state.Gamepad;
                let ref mut gamepad = input.gamepads[index];

                gamepad.connected = state.connected;

                // We can probably factor out a lot of this stuff to `input.rs`
                let deadzone = 0.3;

                gamepad.left_trigger  = s.bLeftTrigger  as f32 / 255.0;
                gamepad.right_trigger = s.bRightTrigger as f32 / 255.0;

                if gamepad.left_trigger < deadzone  { gamepad.left_trigger = 0.0; }
                if gamepad.right_trigger < deadzone { gamepad.right_trigger = 0.0; }

                gamepad.left = Vec2::new(
                    (s.sThumbLX as f32 + 0.5) / 32767.5,
                    (s.sThumbLY as f32 + 0.5) / 32767.5,
                );
                if gamepad.left.len_sqr() < deadzone*deadzone {
                    gamepad.left = Vec2::ZERO;
                }

                gamepad.right = Vec2::new(
                    (s.sThumbRX as f32 + 0.5) / 32767.5,
                    (s.sThumbRY as f32 + 0.5) / 32767.5,
                );
                if gamepad.right.len_sqr() < deadzone*deadzone {
                    gamepad.right = Vec2::ZERO;
                }

                fn update_state(down: bool, gamepad: &mut Gamepad, button: GamepadButton) {
                    let ref mut state = gamepad.buttons[button as usize];

                    if down && !state.down() {
                        *state = KeyState::Pressed;
                    }

                    if !down && state.down() {
                        *state = KeyState::Released;
                    }
                }

                use GamepadButton::*;
                update_state(s.wButtons & 0x0001 != 0, gamepad, DpadUp);
                update_state(s.wButtons & 0x0002 != 0, gamepad, DpadUp);
                update_state(s.wButtons & 0x0004 != 0, gamepad, DpadUp);
                update_state(s.wButtons & 0x0008 != 0, gamepad, DpadUp);
                update_state(s.wButtons & 0x0010 != 0, gamepad, Start);
                update_state(s.wButtons & 0x0020 != 0, gamepad, Back);
                update_state(s.wButtons & 0x0040 != 0, gamepad, LeftStick);
                update_state(s.wButtons & 0x0080 != 0, gamepad, RightStick);
                update_state(s.wButtons & 0x0100 != 0, gamepad, LeftBumper);
                update_state(s.wButtons & 0x0200 != 0, gamepad, RightBumper);
                update_state(s.wButtons & 0x1000 != 0, gamepad, A);
                update_state(s.wButtons & 0x2000 != 0, gamepad, B);
                update_state(s.wButtons & 0x4000 != 0, gamepad, X);
                update_state(s.wButtons & 0x8000 != 0, gamepad, Y);

                let v = 0.8;
                update_state(gamepad.left.y  > v,  gamepad, LeftUp);
                update_state(gamepad.left.y  < -v, gamepad, LeftDown);
                update_state(gamepad.left.x  > v,  gamepad, LeftRight);
                update_state(gamepad.left.x  < -v, gamepad, LeftLeft);
                update_state(gamepad.right.y > v,  gamepad, RightUp);
                update_state(gamepad.right.y < -v, gamepad, RightDown);
                update_state(gamepad.right.x > v,  gamepad, RightRight);
                update_state(gamepad.right.x < -v, gamepad, RightLeft);
                update_state(gamepad.left_trigger  > v, gamepad, LeftTrigger);
                update_state(gamepad.right_trigger > v, gamepad, RightTrigger); 
            }
        }

        fn swap_buffers(&mut self) {
            unsafe { 
                ffi::SwapBuffers(self.device_context); 
            }
        }

        fn close_requested(&self) -> bool { self.close_requested }
        fn resized(&self) -> bool         { self.resized }
        fn moved(&self) -> bool           { self.moved }
        fn focused(&self) -> bool         { self.focused }

        fn screen_region(&self) -> Region { self.screen_region }

        fn change_title(&mut self, title: &str) {
            let title = encode_wide(title);
            unsafe { ffi::SetWindowTextW(self.window, title.as_ptr()) };
        }

        fn set_vsync(&mut self, vsync: bool) {
            if let Some(swap_function) = self.swap_function {
                swap_function(if vsync { 1 } else { 0 });
            } else {
                #[cfg(debug_assertions)]
                println!("`set_vsync` called, but WGL_EXT_swap_control is not supported");
            }
        }

        fn set_cursor(&mut self, cursor: CursorType) {
            self.cursor = cursor;
        }

        fn grab_cursor(&mut self, grabbed: bool) {
            if self.cursor_grabbed == grabbed {
                return;
            }
            self.cursor_grabbed = grabbed;

            self.update_cursor_clip();
        }

        fn clip_cursor(&mut self, region: Option<Region>) {
            self.cursor_clip_region = region;
            self.update_cursor_clip();
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            unsafe { 
                ffi::wglDeleteContext(self.gl_context);
                ffi::DestroyWindow(self.window);
            }
        }
    }

    // Platform specific impls
    impl Window {
        pub fn window_handle(&self) -> ffi::HWND {
            self.window
        }

        fn update_cursor_clip(&self) {
            let mut clip = None;

            if self.focused {
                if self.cursor_grabbed {
                    internal_clip_cursor(Some(self.screen_region));
                } else if let Some(region) = self.cursor_clip_region {
                    clip = Some(region.offset(self.screen_region.min));
                }
            }

            internal_clip_cursor(clip);
        }

        pub fn cursor_in_window(&self) -> bool {
            let mouse_pos = unsafe {
                let mut p = ffi::POINT { x: 0, y: 0 };
                ffi::GetCursorPos(&mut p);
                Vec2::new(p.x, p.y).as_f32()
            };

            self.screen_region.contains(mouse_pos)
        }
    }

    fn new_rect() -> ffi::RECT {
        ffi::RECT { left: 0, right: 0, top: 0, bottom: 0 }
    }

    fn internal_clip_cursor(clip_region: Option<Region>) {
        if let Some(region) = clip_region {
            unsafe {
                let rect = ffi::RECT {
                    left:   region.min.x as i32,
                    right:  region.max.x as i32,
                    top:    region.min.y as i32,
                    bottom: region.max.y as i32,
                };
                ffi::ClipCursor(&rect);
            }
        } else {
            unsafe { ffi::ClipCursor(ptr::null()) };
        }
    }
}
