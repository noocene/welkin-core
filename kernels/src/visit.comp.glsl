#version 450

#include "util.glsl"

void main() {
    u32 i = gl_GlobalInvocationID.x;

    if (i < state.visits_needed) {
        // TODO idx 15?

        if (atomicAdd(state.visits_done, 1) == state.visits_needed - 1) {
            state.visits_needed = 0;
            state.visits_done = 0;
        }

        Index a_addr = needs_visiting[i];
        Agent a = get(a_addr);

        Index initial = a_addr;

        while (a.ty == WIRE) {
            if (a.principal != TEMPORARY && atomicCompSwap(agents[a_addr.data].principal.data, a.principal.data, TEMPORARY.data) == a.principal.data) {
                // TODO free node
            }

            a_addr = index(a.principal);

            if (a_addr == initial) {
                return;
            }

            a = get(a_addr);
        }

        Index b_addr = index(a.principal);
        Agent b = get(b_addr);

        while (b.ty == WIRE) {
            if (b.principal != TEMPORARY && atomicCompSwap(agents[b_addr.data].principal.data, b.principal.data, TEMPORARY.data) == b.principal.data) {
                // TODO free node
            }

            b_addr = index(b.principal);
            b = get(b_addr);
        }

        // TODO slot check
    }
}