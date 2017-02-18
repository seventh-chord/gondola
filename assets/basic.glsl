
-- VERT

#version 330 core

// Inputs are automatically inserted
out vec4 vert_color;

uniform mat4 mvp;

void main() {
    gl_Position = mvp * vec4(position, 0.0, 1.0);
    vert_color = color;
}

-- FRAG

#version 330 core

// Inputs are automatically inserted
out vec4 out_color;

void main() {
    out_color = vert_color;
}

