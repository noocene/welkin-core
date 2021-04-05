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

        Port a_port = port(needs_visiting[i], PRINCIPAL);
        Agent a = follow(a_port);

        Port initial = a_port;

        while (a.ty == WIRE) {
            if (a.principal != TEMPORARY && atomicCompSwap(agents[a_port.data].principal.data, a.principal.data, TEMPORARY.data) == a.principal.data) {
                free(index(a_port));
            }

            a_port = a.principal;

            if (a_port == initial) {
                return;
            }

            a = follow(a_port);
        }

        Port b_port = a.principal;
        Agent b = follow(b_port);

        while (b.ty == WIRE) {
            if (b.principal != TEMPORARY && atomicCompSwap(agents[b_port.data].principal.data, b.principal.data, TEMPORARY.data) == b.principal.data) {
                free(index(b_port));
            }

            b_port = b.principal;
            b = follow(b_port);
        }

        Index a_index = index(a_port);
        Index b_index = index(b_port);

        connect_ports(a_port, b_port);

        if (slot(a_port) == PRINCIPAL && slot(b_port) == PRINCIPAL && a_index.data != 0 && b_index.data != 0) {
            mark_active(index(a_port).data < index(b_port).data ? a_index : b_index);
        }
    }
}