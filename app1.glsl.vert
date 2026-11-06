#version 420

layout(location = 0) in vec4 in_Position;
layout(location = 1) in vec3 in_Colour;

out vec3 Colour;

uniform TransBlock {
    mat4 model;
    mat4 view;
    mat4 projection;
} trb;

void main() {
    gl_Position = trb.projection * (trb.view * (trb.model * in_Position));
    Colour = in_Colour;
}
