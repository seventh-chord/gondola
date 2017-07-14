
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "linux")]
mod linux {
    extern crate x11_dl;

    use std::ptr;
    use std::mem;
    use std::str;
    use std::ffi::CString;

    use gl;

    mod ffi {
        pub(super) use super::x11_dl::xlib::*;
        pub(super) use super::x11_dl::glx::*;
    }

    use cable_math::Vec2;

    use input;

    pub fn test(name: &str) {
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
            ffi::GLX_RGBA, 
            ffi::GLX_DEPTH_SIZE, 24, 
            ffi::GLX_DOUBLEBUFFER, 
            0,
        ];

        let visual = unsafe { (glx.glXChooseVisual)(display, 0, attributes.as_mut_ptr()) };

        if visual.is_null() {
            panic!("No appropriate visual found");
        }

        let mut gl_name_buf = Vec::with_capacity(500);
        gl::load_with(|name| {
            gl_name_buf.clear();
            gl_name_buf.extend_from_slice(name.as_bytes());
            gl_name_buf.push(0);

            unsafe {
                (glx.glXGetProcAddress)(gl_name_buf.as_ptr()).unwrap() as *const _
            }
        });

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

        let mut size = Vec2::new(600.0, 600.0);

        let window = unsafe { (xlib.XCreateWindow)(
            display, root,
            0, 0, 600, 600, // x, y, width, height
            0, // Border

            (*visual).depth, // Depth
            ffi::InputOutput as _,
            (*visual).visual,

            ffi::CWColormap | ffi::CWEventMask,
            &mut win_attributes,
        ) };

        let name = CString::new(name).unwrap();
        unsafe { (xlib.XStoreName)(display, window, name.into_raw()); }

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

        // TODO TEMP
        let mut typed = "".to_owned();

        'main_loop:
        loop {
            // Handle events
            unsafe { while (xlib.XPending)(display) > 0 {
                let mut event = mem::zeroed::<ffi::XEvent>();
                (xlib.XNextEvent)(display, &mut event);
                let ty = event.get_type();

                match ty {
                    ffi::Expose => {
                        // Sent whenever the screen should be redrawn. We can ignore this, since we
                        // continually redraw screen contents anyways.
                    },

                    ffi::KeyPress | ffi::KeyRelease => {
                        // Normal key input
                        let mut event: ffi::XKeyEvent = event.into();

                        /*let prev_state = ;
                        let state = if ty == ffi::KeyPress {
                            input::KeyState::Pressed
                        } else {
                            input::KeyState::Released
                        };
                        */

                        // Typing
                        if ty == ffi::KeyPress {
                            let mut buffer = [0u8; 16];
                            let mut status: ffi::Status = 0;

                            let count = (xlib.Xutf8LookupString)(
                                ic, &mut event,
                                mem::transmute(buffer.as_mut_ptr()),
                                buffer.len() as _,
                                ptr::null_mut(), &mut status,
                            );

                            if status != ffi::XBufferOverflow {
                                let text = str::from_utf8(&buffer[..count as usize]).unwrap_or("");
                                typed.push_str(text);
                            } else {
                                // Try again with a dynamic buffer
                                let mut buffer = vec![0u8; count as usize];
                                let count = (xlib.Xutf8LookupString)(
                                    ic, &mut event,
                                    mem::transmute(buffer.as_mut_ptr()),
                                    buffer.len() as _,
                                    ptr::null_mut(), &mut status
                                );

                                let text = str::from_utf8(&buffer[..count as usize]).unwrap_or("");
                                typed.push_str(text);
                            }
                        }
                    },

                    // Mouse buttons
                    ffi::ButtonPress => {
                        let event: ffi::XButtonEvent = event.into();

                        println!("press {} {}", event.button, event.state);
                    },
                    ffi::ButtonRelease => {
                        let event: ffi::XButtonEvent = event.into();

                        println!("release {} {}", event.button, event.state);
                    },

                    // Mouse movement
                    ffi::MotionNotify => {
                        let event: ffi::XMotionEvent = event.into();

                        let new_pos = Vec2::new(event.x, event.y).as_f32();
                    },

                    ffi::MappingNotify => {
                        (xlib.XRefreshKeyboardMapping)(event.as_mut());
                    },

                    ffi::ConfigureNotify => {
                        let event: ffi::XConfigureEvent = event.into();

                        let new_size = Vec2::new(event.width, event.height).as_f32();

                        if new_size != size {
                            size = new_size;

                            println!("Resized");
                        }
                    },
                    ffi::ReparentNotify => {},
                    ffi::MapNotify => {},

                    ffi::ClientMessage => {
                        let event: ffi::XClientMessageEvent = event.into();

                        if event.data.get_long(0) == wm_delete_window as i64 {
                            println!("We darn well got a delete message!");
                            break 'main_loop;
                        }
                    },

                    other => {
                        panic!("Unkown X event type: {}", other);
                    },
                }
            } }
            // End of event handling

//            println!("No more events");
//            ::std::thread::sleep(::std::time::Duration::from_millis(500));
        }

        // Cleanup
        unsafe {
            (xlib.XDestroyIC)(ic);
            (xlib.XCloseIM)(im);

            (xlib.XDestroyWindow)(display, window);
            (xlib.XCloseDisplay)(display);
        }
    }
}
