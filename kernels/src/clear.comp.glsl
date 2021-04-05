#version 450

#include "util.glsl"

void main() {
    agents[gl_GlobalInvocationID.x] = FREE;
}