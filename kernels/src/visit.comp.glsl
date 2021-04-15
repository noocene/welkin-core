#version 450

#include "util.glsl"

void main() {
    u32 i = gl_GlobalInvocationID.x;

    if (i < state.visits_needed) {
        if (atomicAdd(state.visits_done, 1) == state.visits_needed - 1) {
            state.visits_needed = 0;
            state.visits_done = 0;
        }

        Port a_port = needs_visiting[i];
        Agent a = follow(a_port);

        Port initial = a_port;

        while (a.ty == WIRE) {
            Index a_port_idx = index(a_port);

            if (a.principal != TEMPORARY && atomicCompSwap(agents[a_port_idx.data].principal.data, a.principal.data, TEMPORARY.data) == a.principal.data) {
                free(index(a_port));
            }

            a_port = through(a_port);
            a = follow(a_port);

            if (a_port == initial) {
                return;
            }
        }

        Port b_port = through(a_port);
        Agent b = follow(b_port);

        while (b.ty == WIRE) {
            Index b_port_idx = index(b_port);
            
            if (b.principal != TEMPORARY && atomicCompSwap(agents[b_port_idx.data].principal.data, b.principal.data, TEMPORARY.data) == b.principal.data) {
                free(index(b_port));
            }

            b_port = through(b_port);
            b = follow(b_port);
        }

        Index a_index = index(a_port);
        Index b_index = index(b_port);

        connect_ports(a_port, b_port);

        if (slot(a_port) == PRINCIPAL && slot(b_port) == PRINCIPAL && a.ty != ROOT && b.ty != ROOT) {
            mark_active(index(a_port).data < index(b_port).data ? a_index : b_index);
        }
    }
}