#define u32 uint

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

struct Port {
    u32 data;
};

const Port TEMPORARY = Port(0xFFFFFFFF);

struct Ty {
    u32 data;
};

struct Agent {
    Port principal;
    Port left;
    Port right;
    Ty ty;
};

bool same_ty(Agent a, Agent b) {
    return a.ty == b.ty;
}

struct Index {
    u32 data;
};

layout(set = 0, binding = 0) buffer Agents {
   Agent agents[];
};

Index index(Port port) {
    return Index(port.data >> 2);
}

Agent follow(Port port) {
    return agents[index(port).data];
}

Agent get(Index index) {
    return agents[index.data];
}

struct Slot {
    u32 data;
};

const Slot PRINCIPAL = Slot(0);
const Slot LEFT = Slot(1);
const Slot RIGHT = Slot(2);

Slot slot(Port port) {
    return Slot(port.data & 3);
}

layout(set = 0, binding = 1) buffer ActiveAgents {
   Index active_agents[];
};

layout(set = 0, binding = 2) buffer FreedAgents {
   Index freed_agents[];
};

layout(set = 0, binding = 3) buffer NeedsVisitingAgents {
   Port needs_visiting[];
};

layout(set = 0, binding = 4) buffer State {
    u32 agents;
    u32 active_pairs;
    u32 active_pairs_done;
    u32 freed_agents;
    u32 visits_needed;
    u32 visits_done;
    u32 rewrites;
} state;

Port port(Index index, Slot slot) {
    return Port((index.data << 2) | slot.data);
}

Slot principal() {
    return Slot(0);
}

void replace(Index index, Agent agent) {
    agents[index.data] = agent;
}

const Ty EPSILON = Ty(0);
const Ty DELTA = Ty(1);
const Ty ZETA = Ty(2);
const Ty ROOT = Ty(3);
const Ty WIRE = Ty(0xFFFFFFFF);

Index alloc() {
    u32 pos = atomicAdd(state.freed_agents, -1);
    return pos > 0 && pos < 0x80000000 ? freed_agents[pos - 1] : Index(atomicAdd(state.agents, 1));
}

const Port FREE_PORT = Port(0xFFFFFFFF << 2);

const Agent FREE = Agent(
    FREE_PORT,
    FREE_PORT,
    FREE_PORT,
    WIRE
);

void mark_for_visit(Port port) {
    needs_visiting[atomicAdd(state.visits_needed, 1)] = port;
}

void mark_active(Index index) {
    active_agents[atomicAdd(state.active_pairs, 1)] = index;
}

void free(Index index) {
    freed_agents[atomicAdd(state.freed_agents, 1)] = index;
}

void connect_ports(Port a, Port b) {
    switch (slot(a).data) {
        case PRINCIPAL.data:
            agents[index(a).data].principal = b;
            break;
        case LEFT.data:
            agents[index(a).data].left = b;
            break;
        case RIGHT.data:
            agents[index(a).data].right = b;
            break;
    }
    switch (slot(b).data) {
        case PRINCIPAL.data:
            agents[index(b).data].principal = a;
            break;
        case LEFT.data:
            agents[index(b).data].left = a;
            break;
        case RIGHT.data:
            agents[index(b).data].right = a;
            break;
    }
}

Port through(Port port) {
    Agent agent = follow(port);
    Port ret = TEMPORARY;
    switch (slot(port).data) {
        case PRINCIPAL.data:
            ret = agent.principal;
            break;
        case LEFT.data:
            ret = agent.left;
            break;
        case RIGHT.data:
            ret = agent.right;
            break;
    }
    return ret;
}