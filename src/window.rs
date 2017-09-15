
use cable_math::Vec2;

use Region;
use input::{KeyState, InputManager};
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

    fn poll_events(&mut self, input: &mut InputManager);
    fn swap_buffers(&mut self);

    fn close_requested(&self) -> bool;
    fn resized(&self) -> bool;
    fn moved(&self) -> bool;
    fn screen_region(&self) -> Region;
    fn focused(&self) -> bool;

    fn change_title(&mut self, title: &str);
    /// Enables/disables vsync, if supported by the graphics driver. In debug mode a warning is
    /// printed when calling this function if changing vsync is not supported. By default, vsync is
    /// disabled.
    fn set_vsync(&mut self, vsync: bool);

    fn set_cursor(&mut self, cursor: CursorType);
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
        cursor: CursorType,
        focused: bool,

        region: Region,
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
            let size = Vec2::new(640.0, 480.0);
            let region = Region {
                min: center/2.0 - size/2.0,
                max: center/2.0 + size/2.0,
            };

            let window = unsafe { (xlib.XCreateWindow)(
                display, root,
                region.min.x as i32, region.min.y as i32,
                region.width() as u32, region.height() as u32,
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

            graphics::viewport(region.unpositioned());

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
                region,

                close_requested: false,
                resized: false,
                moved: false,
                cursor_grabbed: false,
                cursor: CursorType::Normal,
                focused: false,
            }
        }

        fn show(&mut self) {
            unsafe { (self.xlib.XMapWindow)(self.display, self.window); }
        }

        fn poll_events(&mut self, input: &mut InputManager) {
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
                        input.focused = self.focused;
                    },

                    ffi::FocusOut => {
                        self.internal_grab_cursor(false);
                        self.internal_set_cursor(CursorType::Normal);

                        self.focused = false;
                        input.focused = self.focused;
                    },

                    ffi::KeyPress | ffi::KeyRelease => {
                        input.changed = true;
                        let mut event: ffi::XKeyEvent = event.into();

                        // Normal key input
                        let scancode = event.keycode;

                        let ref mut state = input.keyboard_states[scancode as usize];
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
                        input.changed = true;

                        let event: ffi::XButtonEvent = event.into();

                        let state = if ty == ffi::ButtonPress {
                            KeyState::Pressed
                        } else {
                            KeyState::Released
                        };

                        match event.button {
                            // X11 uses different button indices
                            1 => input.mouse_states[0] = state,
                            2 => input.mouse_states[2] = state,
                            3 => input.mouse_states[1] = state,
                            
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
                        input.changed = true;

                        let event: ffi::XMotionEvent = event.into();

                        let new_pos = Vec2::new(event.x, event.y).as_f32();
                        if new_pos != input.mouse_pos {
                            let delta = new_pos - input.mouse_pos;
                            input.mouse_delta += delta;
                            input.raw_mouse_delta += delta;

                            input.mouse_pos = new_pos;
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

                        if new_region.min != self.region.min {
                            self.moved = true;
                        }

                        if new_region.size() != self.region.size() {
                            self.resized = true;
                        }

                        self.region = new_region;
                        graphics::viewport(self.region.unpositioned());
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

            // Constrain cursor if it is grabbed
            if self.cursor_grabbed && self.focused {
                let center = self.region.unpositioned().center().as_i32();
                input.mouse_pos = center.as_f32();

                unsafe {
                    (self.xlib.XWarpPointer)(
                        self.display, 0, self.window,
                        0, 0, 0, 0,
                        center.x, center.y,
                    );
                    (self.xlib.XFlush)(self.display);
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
        fn screen_region(&self) -> Region   { self.region }

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
        mouse_captured: bool, // Cursor is dragging something out of the window, don't loose focus on release
        mouse_grabbed: bool, // Cursor cant leave window
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
        Resized(Vec2<f32>),
        Moved(Vec2<f32>),
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
            ffi::WM_SIZE => {
                let width = ffi::LOWORD(l as ffi::DWORD) as u32;
                let height = ffi::HIWORD(l as ffi::DWORD) as u32;
                Some(RawEvent::Resized(Vec2::new(width, height).as_f32()))
            },

            ffi::WM_MOVE => {
                let x = ffi::LOWORD(l as ffi::DWORD) as u32;
                let y = ffi::HIWORD(l as ffi::DWORD) as u32;
                Some(RawEvent::Moved(Vec2::new(x, y).as_f32()))
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
                    bytes.as_ptr() as *mut _, &mut size,
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
                let mut rect = mem::zeroed::<ffi::RECT>();
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
            let lib_name = encode_wide("opengl32.dll");
            let gl32_lib = unsafe { ffi::LoadLibraryW(lib_name.as_ptr()) };
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
                            extensions are supported. Your drivers/the spec suck"
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
                mouse_captured: false,
                mouse_grabbed: false,
            }
        } 

        fn show(&mut self) {
            unsafe { ffi::ShowWindow(self.window, ffi::SW_SHOW) };
        }

        fn poll_events(&mut self, input: &mut InputManager) {
            let focused = unsafe { ffi::GetFocus() == self.window };
            let focus_changed = self.focused != focused;
            self.focused = focused;
            input.focused = self.focused;

            // Receive events from windows, dispatch them to `event_callback` and let them get sent
            // back through `raw_event_receiver`.
            let mut msg = unsafe { mem::uninitialized::<ffi::MSG>() };
            loop {
                let result = unsafe { ffi::PeekMessageW(
                        &mut msg, self.window, 
                        0, 0,
                        ffi::PM_REMOVE,
                        ) };

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
                    Resized(new_size) => {
                        self.screen_region.max = self.screen_region.min + new_size;
                        self.resized = true;

                        graphics::viewport(self.screen_region.unpositioned());
                    },

                    Moved(new_pos) => {
                        let size = self.screen_region.size();

                        self.screen_region = Region {
                            min: new_pos,
                            max: new_pos + size,
                        };

                        self.moved = true;
                    },

                    CloseRequest => {
                        self.close_requested = true;
                    },

                    Key(pressed, code) => {
                        input.changed = true;

                        let ref mut state = input.keyboard_states[code];
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
                        input.changed = true;

                        for result in char::decode_utf16([wchar].iter().cloned()) {
                            match result {
                                Ok(c) => input.type_buffer.push(c),
                                Err(_) => println!("WM_CHAR with invalid code: {}", wchar),
                            }
                        }
                    },

                    Scroll(delta) => {
                        input.changed = true;
                        input.mouse_scroll += delta;
                    },

                    MousePos(new_pos) => {
                        if new_pos != input.mouse_pos {
                            input.changed = true;

                            input.mouse_delta += new_pos - input.mouse_pos;
                            input.mouse_pos = new_pos;
                        }
                    },

                    MouseDelta(delta) => {
                        if delta != Vec2::zero() {
                            input.changed = true;
                            input.raw_mouse_delta += delta;
                        }
                    },

                    MouseButton(down, code) => {
                        input.changed = true;

                        let state = if down { KeyState::Pressed } else { KeyState::Released };
                        input.mouse_states[code] = state;

                        let mut any_down = false;
                        for state in input.mouse_states.iter() {
                            if state.down() {
                                any_down = true;
                                break;
                            }
                        }

                        // As long as any mouse buttons are down we want to capture the mouse. This
                        // allows draging stuff around to work even when the mouse temporarily
                        // leaves the window.
                        let mouse_captured = any_down;
                        if mouse_captured != self.mouse_captured {
                            self.mouse_captured = mouse_captured;
                            if self.mouse_captured {
                                unsafe { ffi::SetCapture(self.window) };
                            } else {
                                unsafe { ffi::ReleaseCapture() };
                            }
                        }
                    },
                }
            }

            if focus_changed {
                self.clip_cursor(self.mouse_grabbed && self.focused);
            }

            if self.focused && self.mouse_grabbed {
                let global_center = self.screen_region.center().as_i32();
                let relative_center = self.screen_region.unpositioned().center().as_i32();
                input.mouse_pos = relative_center.as_f32();
                unsafe { ffi::SetCursorPos(global_center.x, global_center.y) };
            }
        }

        fn swap_buffers(&mut self) {
            unsafe { 
                ffi::SwapBuffers(self.device_context);

                if self.window_hovered() {
                    let cursor = self.cursors[self.cursor as usize];
                    ffi::SetCursor(cursor);
                }
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
            if self.mouse_grabbed == grabbed {
                return;
            }
            self.mouse_grabbed = grabbed;

            if self.focused {
                self.clip_cursor(grabbed);
            }
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

        fn window_hovered(&self) -> bool {
            let mouse_pos = unsafe {
                let mut p = ffi::POINT { x: 0, y: 0 };
                ffi::GetCursorPos(&mut p);
                Vec2::new(p.x, p.y).as_f32()
            };

            self.screen_region.contains(mouse_pos)
        }

        fn clip_cursor(&self, clip: bool) {
            if clip {
                unsafe {
                    let region = self.screen_region;
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
}
