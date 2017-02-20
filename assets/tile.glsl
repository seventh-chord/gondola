

-- VERT
#version 330 core

// Inputs are automatically inserted

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}

-- GEOM
#version 330 core

layout(points) in;
layout(triangle_strip, max_vertices=6) out;

out vec2 tex_coords;

uniform vec2 size;
uniform mat4 mvp;

void main() {
    vec4 pos = gl_in[0].gl_Position;

    gl_Position = mvp * pos;
    tex_coords = vec2(0, 1);
    EmitVertex();
    gl_Position = mvp * (pos + vec4(size.x, 0, 0, 0));
    tex_coords = vec2(1, 1);
    EmitVertex();
    gl_Position = mvp * (pos + vec4(size.x, size.y, 0, 0));
    tex_coords = vec2(1, 0);
    EmitVertex();
    EndPrimitive();

    gl_Position = mvp * pos;
    tex_coords = vec2(0, 1);
    EmitVertex();
    gl_Position = mvp * (pos + vec4(size.x, size.y, 0, 0));
    tex_coords = vec2(1, 0);
    EmitVertex();
    gl_Position = mvp * (pos + vec4(0, size.y, 0, 0));
    tex_coords = vec2(0, 0);
    EmitVertex();
    EndPrimitive();
}

-- FRAG
#version 330 core

//Inputs are automatically inserted
out vec4 out_color;

uniform sampler2D tex_sampler;

void main() {
    out_color = texture2D(tex_sampler, tex_coords);
}

