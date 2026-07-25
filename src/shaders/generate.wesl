// camera ray generation
// camera data -> ray buffer + active indices+counter

// import super::types::Ray;
import super::types::Vars;

// requires readonly_and_readwrite_storage_textures;

@group(0) @binding(0)
var<uniform> vars: Vars;
@group(0) @binding(1)
var output: texture_storage_2d<rgba32float, read_write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // let ray = Ray();
    let coords = vec2i(id.xy);

    textureStore(output, coords, vec4f(f32(coords.x) / vars.size.x, f32(coords.y) / vars.size.y, 0.0, 1.0));
}