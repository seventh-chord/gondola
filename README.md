
# Gondola

A cross-platform<sup>\*</sup> window and OpenGL 3.3 rendering library. Though this library is
primarily aimed at creating games, it can also be used for other kinds of applications. A variety
of utilities are provided to abstract over OpenGL concepts and facilitate common tasks such as 
font rendering. Note that this library currently is not able to do everything raw OpenGL can.

This library is still in development, so major redesigns should be expected. As a result, the
library is not available on `crates.io` yet.

<sup>\*</sup>Currently, only windows and linux are supported, as I do not have access to a 
machine running macos.


## Getting started

The following is a minimal example which creates a window, retrieves input and swaps the backbuffer.

```
use gondola::{Window, WindowCommon, InputManager};

fn main() {
    let mut input = InputManager::new();
    let mut window = Window::new("My title");

    while !window.close_requested {
        window.poll_events(input);

        // Update and render here!

        window.swap_buffers();
    }
}
```

A more complete example can be found in `src/bin/windo_demo.rs`. Note that the code in that file 
only showcases the `DrawGroup` struct for rendering 2D graphics. More complex graphics can be drawn
by using custom `Shader`s and `VertexBuffer`s.

## cable\_math

A second crate lives in the `cable_math` directory. This crate is a simple vector, matrix and 
quaternion math library, which is used in `gondola`. The crate supports 2D, 3D and 4D vectors,
2x2, 3x3 and 4x4 matricies, and quaternions.

## gondola\_derive

This library simply provides a procedural macro to generate a implementation of the `Vertex` trait
for arbitrary `struct`s. This is generally needed when using `VertexBuffer`s. More information on 
how to use this macro is contained in the documentation for `VertexBuffer`.

## Why use custom code to create a window?

There are many good libraries for setting up a window and creating a OpenGL context, such as 
`glutin` and `winit`. In spite of this, a custom solution is used in this library. The primary
argument in favour of this is that other libraries don't provide all the functionality I need
for my applications. For example, at the time of writing this none of the major libraries support
constraining the cursor to a subregion of the window. Because these libraries support so many
platforms, it is very difficult to add such features in a maner which works consistently across
all supported platforms.

Furthermore, these libraries are seemingly very complex. The entirety of the window creation and 
management code of this library is contained in a single ~1.5K line file (`src/window.rs`). Compare
this to `winit`s >10K lines. This is not to say that our approach is better, but it serves to show 
that window code not neccesarily has to be very complex.

Of course, there are disadvantages with choosing this approach: We can not support as many 
platforms, we probably miss certain edge-cases, and we have to manintain an aditional piece of 
code. For the time being, the benefits of using a custom solution seem to outweight the problems.
This might however change in the future.
