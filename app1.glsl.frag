#version 420

in vec3 Colour;

out vec4 out_Colour;

uniform TransBlock {
    mat4 model;
    mat4 view;
    mat4 projection;
} trb;

void main() {
    out_Colour = vec4(Colour, 1.);
}
