// format indirect dispatch buffer, swap counters

import super::types::{Vars, Indirect};

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var<storage, read_write> active_ray_counter: atomic<i32>;
@group(0) @binding(2)
var<storage, read_write> active_ray_indirect: Indirect;

@compute @workgroup_size(1)
fn main() {
    let usize = vec2u(vars.size);

    active_ray_indirect.count = atomicLoad(&active_ray_counter);
    atomicStore(&active_ray_counter, 0);

    active_ray_indirect.dim_x = (i32(active_ray_indirect.count) + 63) / 64;
    active_ray_indirect.dim_y = 1;
    active_ray_indirect.dim_z = 1;
}