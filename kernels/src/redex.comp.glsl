#version 450

#include "util.glsl"

void main() {
    u32 i = gl_GlobalInvocationID.x;

    if (i < state.active_pairs) {
        Index a_addr = active_agents[i];
        Agent a = get(a_addr);

        if (a.ty != WIRE && atomicCompSwap(agents[a_addr.data].ty.data, a.ty.data, WIRE.data) == a.ty.data) {
            atomicAdd(state.rewrites, 1);
            Index b_addr = index(a.principal);
            Agent b = get(b_addr);

            if (!same_ty(a, b)) {
                Index p_addr = alloc();
                Index q_addr = alloc();
                Index r_addr = alloc();
                Index s_addr = alloc();

                replace(p_addr, Agent(
                    b.left,
                    port(s_addr, LEFT),
                    port(r_addr, LEFT),
                    a.ty
                ));
                replace(q_addr, Agent(
                    b.right,
                    port(s_addr, RIGHT),
                    port(r_addr, RIGHT),
                    a.ty
                ));

                replace(r_addr, Agent(
                    a.left,
                    port(p_addr, LEFT),
                    port(q_addr, LEFT),
                    b.ty
                ));
                replace(r_addr, Agent(
                    a.right,
                    port(p_addr, RIGHT),
                    port(q_addr, RIGHT),
                    b.ty
                ));

                mark_for_visit(index(a.left));
                mark_for_visit(index(a.right));
                mark_for_visit(index(b.left));
                mark_for_visit(index(b.right));
            } else {
                replace(a_addr, Agent(
                    a.principal,
                    b.left,
                    b.right,
                    WIRE
                ));
                replace(b_addr, Agent(
                    b.principal,
                    a.left,
                    a.right,
                    WIRE
                ));

                mark_for_visit(index(a.left));
                mark_for_visit(index(a.right));
            }
        }

        if (atomicAdd(state.active_pairs_done, 1) == state.active_pairs - 1) {
            state.active_pairs_done = 0;
            state.active_pairs = 0;
            state.freed_agents = state.freed_agents > 0x80000000 ? 0 : state.freed_agents;
        }
    }
}