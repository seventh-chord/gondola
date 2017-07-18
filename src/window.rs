
use cable_math::Vec2;

use Region;
use input::{KeyState, InputManager};
use graphics;

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

pub trait Window: Drop {
    fn show(&mut self);

    fn poll_events(&mut self, input: &mut InputManager);
    fn swap_buffers(&mut self);

    fn close_requested(&self) -> bool;
    fn resized(&self) -> bool;
    fn moved(&self) -> bool;

    fn screen_region(&self) -> Region;

    fn change_title(&mut self, title: &str);
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
    }

    pub struct X11Window {
        xlib: ffi::Xlib,
        glx: ffi::Glx,

        display: *mut ffi::Display,
        window: u64,

        im: ffi::XIM,
        ic: ffi::XIC,

        wm_delete_window: ffi::Atom,

        close_requested: bool,
        resized: bool,
        moved: bool,

        region: Region,
    }

    pub fn new_window(title: &str, gl_request: GlRequest) -> X11Window {
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
                ffi::ButtonPressMask | ffi::ButtonReleaseMask,

            colormap: colormap,

            .. unsafe { mem::zeroed() }
        };

        let region = Region {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(600.0, 600.0),
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

        // Finish setting up OpenGL
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
            let mut atom = (xlib.XInternAtom)(display, b"WM_DELETE_WINDOW\0".as_ptr() as *const _, 0);
            (xlib.XSetWMProtocols)(display, window, &mut atom, 1);
            atom
        };

        X11Window {
            xlib, glx,
            display,
            window,
            im,
            ic,
            wm_delete_window,
            region,

            close_requested: false,
            resized: false,
            moved: false,
        }
    }

    impl Window for X11Window {
        fn show(&mut self) {
            unsafe { (self.xlib.XMapWindow)(self.display, self.window); }
        }

        fn poll_events(&mut self, input: &mut InputManager) {
            input.refresh();

            self.moved = false;
            self.resized = false;
            self.close_requested = false;

            let ref xlib = self.xlib;

            // Handle events
            unsafe { while (xlib.XPending)(self.display) > 0 {
                let mut event = mem::zeroed::<ffi::XEvent>();
                (xlib.XNextEvent)(self.display, &mut event);
                let ty = event.get_type();

                match ty {
                    ffi::Expose => {
                        // Sent whenever the screen should be redrawn. We can ignore this, since we
                        // continually redraw screen contents anyways.
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

                            let count = (xlib.Xutf8LookupString)(
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
                                let count = (xlib.Xutf8LookupString)(
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
                            input.mouse_delta += new_pos - input.mouse_pos;
                            input.mouse_pos = new_pos;
                        }
                    },

                    ffi::MappingNotify => {
                        (xlib.XRefreshKeyboardMapping)(event.as_mut());
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
        }

        fn swap_buffers(&mut self) {
            let ref glx = self.glx;

            unsafe {
                (glx.glXSwapBuffers)(self.display, self.window);
            }
        }

        fn close_requested(&self) -> bool { self.close_requested }
        fn resized(&self) -> bool { self.resized }
        fn moved(&self) -> bool { self.resized }
        fn screen_region(&self) -> Region { self.region }

        fn change_title(&mut self, title: &str) {
            let title = CString::new(title).unwrap();
            unsafe { (self.xlib.XStoreName)(self.display, self.window, title.into_raw()); }
        }
    }

    impl Drop for X11Window {
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

    use gl;

    // We access all ffi stuff through `ffi::whatever` instead of through each apis specific
    // bindings. This allows us to easily add custom stuff that is missing in bindings.
    mod ffi {
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
    }

    pub struct WindowsWindow {
        raw_event_receiver: mpsc::Receiver<RawEvent>,
        device_context: ffi::HDC,
        gl_context: ffi::HGLRC,
        window: ffi::HWND,

        screen_region: Region,
        close_requested: bool,
        resized: bool,
        moved: bool,
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

    pub fn new_window(title: &str, gl_request: GlRequest) -> WindowsWindow {
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

        // Actually create window
        // TODO this is really shoddy
        let pos = Vec2::new(20.0, 20.0);
        let size = Vec2::new(640.0, 480.0);
        let region = Region {
            min: pos,
            max: pos + size,
        };

        let window = unsafe { ffi::CreateWindowExW(
            // Extended style
            0, 

            class_name.as_ptr(),
            window_name.as_ptr(),

            ffi::WS_OVERLAPPEDWINDOW,

            region.min.x as i32, region.min.y as i32,
            region.width() as i32, region.height() as i32,

            ptr::null_mut(), // Parent
            ptr::null_mut(), // Menu
            instance,
            ptr::null_mut(), // lParam
        ) };
        if window.is_null() {
            panic!("Failed to create window");
        } 

        let device_context = unsafe { ffi::GetDC(window) };

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
                if address.is_null() {
                    // This is needed for some pre gl 3 functions
                    kernel32::GetProcAddress(gl32_lib, gl_name_buf.as_ptr() as *const _)
                } else {
                    address
                }
            }
        };

        #[allow(non_camel_case_types)]
        type wglCreateContextAttribsARBType = extern "system" fn(
            ffi::HDC,
            ffi::HGLRC,
            *const i32,
        ) -> ffi::HGLRC;

        // TODO maybe query extensions?

        #[allow(non_snake_case)]
        let wglCreateContextAttribsARB = unsafe {
            let p = get_proc_address("wglCreateContextAttribsARB");
            mem::transmute::<_, wglCreateContextAttribsARBType>(p)
        };
        // TODO check if this returned null

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

        WindowsWindow {
            raw_event_receiver,
            device_context,
            gl_context,
            window,

            screen_region: region,
            close_requested: false,
            resized: false,
            moved: false,
        }
    } 

    #[derive(Debug, Copy, Clone)]
    enum RawEvent {
        Resized(Vec2<f32>),
        Moved(Vec2<f32>),
        CloseRequest,
        Key(bool, usize),
        Char(u16),
        Scroll(f32),
        MouseMove(Vec2<f32>),
        MouseButton(bool, usize),
    }

    thread_local! {
        static MSG_SENDER: RefCell<Option<mpsc::Sender<RawEvent>>> = RefCell::new(None);
    }

    // This is WNDPROC
    unsafe extern "system" 
    fn event_callback(window: ffi::HWND, msg: u32, w: ffi::WPARAM, l: ffi::LPARAM) -> ffi::LRESULT { 
        let event = match msg {
            ffi::WM_SIZE => {
                let width = ffi::LOWORD(l as ffi::DWORD) as u32;
                let height = ffi::HIWORD(l as ffi::DWORD) as u32;
                RawEvent::Resized(Vec2::new(width, height).as_f32())
            },

            ffi::WM_MOVE => {
                let x = ffi::LOWORD(l as ffi::DWORD) as u32;
                let y = ffi::HIWORD(l as ffi::DWORD) as u32;
                RawEvent::Moved(Vec2::new(x, y).as_f32())
            },

            ffi::WM_CLOSE => {
                RawEvent::CloseRequest
            },

            ffi::WM_KEYUP | ffi::WM_KEYDOWN => {
                let down = msg == ffi::WM_KEYDOWN;
                let scancode = ((l as usize) >> 16) & 0xff;
                RawEvent::Key(down, scancode)
            },

            ffi::WM_CHAR => {
                RawEvent::Char(w as u16)
            },

            ffi::WM_MOUSEWHEEL => {
                let delta = ffi::GET_WHEEL_DELTA_WPARAM(w) as f32 / ffi::WHEEL_DELTA as f32;
                RawEvent::Scroll(delta)
            },

            ffi::WM_MOUSEMOVE => {
                let x = ffi::GET_X_LPARAM(l);
                let y = ffi::GET_X_LPARAM(l);
                let pos = Vec2::new(x, y).as_f32();
                RawEvent::MouseMove(pos)
            },

            ffi::WM_LBUTTONDOWN => RawEvent::MouseButton(true, 0),
            ffi::WM_LBUTTONUP   => RawEvent::MouseButton(false, 0),
            ffi::WM_MBUTTONDOWN => RawEvent::MouseButton(true, 2),
            ffi::WM_MBUTTONUP   => RawEvent::MouseButton(false, 2),
            ffi::WM_RBUTTONDOWN => RawEvent::MouseButton(true, 1),
            ffi::WM_RBUTTONUP   => RawEvent::MouseButton(false, 1),

            _ => return ffi::DefWindowProcW(window, msg, w, l), // Maybe we don't need this
        };

        MSG_SENDER.with(|sender| {
            if let Some(ref sender) = *sender.borrow() {
                sender.send(event).unwrap();
            } else {
                panic!("`event_callback` called from unkown thread");
            }
        });

        return 0;
    }

    impl Window for WindowsWindow {
        fn show(&mut self) {
            unsafe { ffi::ShowWindow(self.window, ffi::SW_SHOW) };
        }

        fn poll_events(&mut self, input: &mut InputManager) {
            // Receive events from windows, dispatch them to `event_callback` and let them get sent
            // back through `raw_event_receiver`.
            let mut msg = unsafe { mem::uninitialized::<ffi::MSG>() };
            loop {
                let result = unsafe { ffi::PeekMessageW(
                    &mut msg, self.window, 
                    0, 0,
                    ffi::PM_REMOVE,
                ) };

                match result {
                    -1 => panic!("PeekMessage returned -1"), // TODO check if this can happen
                    0 => break,

                    _ => unsafe {
                        ffi::TranslateMessage(&mut msg);
                        ffi::DispatchMessageW(&mut msg);
                    },
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

                    Key(down, code) => {
                        let ref mut state = input.keyboard_states[code];
                        *state = if down {
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
                        for result in char::decode_utf16([wchar].iter().cloned()) {
                            match result {
                                Ok(c) => input.type_buffer.push(c),
                                Err(_) => println!("WM_CHAR with invalid code: {}", wchar),
                            }
                        }
                    },

                    Scroll(delta) => {
                        input.mouse_scroll += delta;
                    },

                    MouseMove(new_pos) => {
                        let delta = new_pos - input.mouse_pos;
                        input.mouse_delta += delta;
                        input.mouse_pos = new_pos;
                    },

                    MouseButton(down, code) => {
                        let state = if down { KeyState::Pressed } else { KeyState::Released };
                        input.mouse_states[code] = state;
                    },
                }
            }
        }

        fn swap_buffers(&mut self) {
            unsafe { ffi::SwapBuffers(self.device_context) };
        }

        fn close_requested(&self) -> bool { self.close_requested }
        fn resized(&self) -> bool         { self.resized }
        fn moved(&self) -> bool           { self.moved }

        fn screen_region(&self) -> Region { self.screen_region }

        fn change_title(&mut self, title: &str) {
            let title = encode_wide(title);
            unsafe { ffi::SetWindowTextW(self.window, title.as_ptr()) };
        }
    }

    impl Drop for WindowsWindow {
        fn drop(&mut self) {
            unsafe { 
                ffi::wglDeleteContext(self.gl_context);
                ffi::DestroyWindow(self.window);
            }
        }
    }
}
