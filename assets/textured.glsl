
-- VERT

#version 330 core

// Inputs are automatically inserted
out vec2 vert_tex;

uniform mat4 mvp;

void main() {
    gl_Position = mvp * vec4(position, 0.0, 1.0);
    vert_tex = tex_coord;
}

-- FRAG

#version 330 core

//Inputs are automatically inserted
out vec4 out_color;

uniform sampler2D tex_sampler;

void main() {
    out_color = texture2D(tex_sampler, vert_tex);
}

