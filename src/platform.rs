
use cable_math::Vec2;

use Region;
use input::{KeyState, InputManager};

pub trait Window: Drop {
    fn poll_events(&mut self, input: &mut InputManager);
    fn swap_buffers(&mut self);

    fn close_requested(&self) -> bool;
    fn resized(&self) -> bool;

    fn screen_region(&self) -> Region;
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
    use std::ffi::{CStr, CString};

    use gl;

    mod ffi {
        pub(super) use super::x11_dl::xlib::*;
        pub(super) use super::x11_dl::glx::*;
        pub(super) use super::x11_dl::glx::arb::*;
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

        region: Region,
    }

    pub fn new_window(name: &str) -> X11Window {
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

//            ffi::GLX_CONTEXT_MAJOR_VERSION_ARB, 3,
//            ffi::GLX_CONTEXT_MINOR_VERSION_ARB, 3,
//            ffi::GLX_CONTEXT_CORE_PROFILE_BIT_ARB,  1,

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

        let name = CString::new(name).unwrap();
        unsafe { (xlib.XStoreName)(display, window, name.into_raw()); }

        // Finish setting up OpenGL
        let context = unsafe {
            let context = (glx.glXCreateContext)(
                display, visual, ptr::null_mut(), 1
            );

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
            let mut raw = gl::GetString(gl::VERSION);
            let version = CStr::from_ptr(raw as *const _).to_string_lossy();
            println!("{}", version);
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

        // Show window
        unsafe { (xlib.XMapWindow)(display, window); }

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
            resized: true,
        }
    }

    impl Window for X11Window {
        fn poll_events(&mut self, input: &mut InputManager) {
            input.refresh();

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

                        let new_region = Region {
                            min: Vec2::new(event.x, event.y).as_f32(),
                            max: Vec2::new(event.x + event.width, event.y + event.height).as_f32(),
                        };

                        if new_region != self.region {
                            self.region = new_region;
                            self.resized = true;
                        }
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
            let ref xlib = self.xlib;
            let ref glx = self.glx;

            // TODO
        }

        fn close_requested(&self) -> bool {
            self.close_requested
        }

        fn resized(&self) -> bool {
            self.resized
        }

        fn screen_region(&self) -> Region {
            self.region
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
        display: *mut ffi::Display,
        event: *mut ffi::XErrorEvent
    ) -> i32
    {
        println!("X error: {}", (*event).error_code);
        0
    }
}
