// camera ray generation
// camera data -> ray buffer + active indices+counter

import super::types::Ray;

@compute @workgroup_size(64)
fn compute() {
    let ray = Ray();
}