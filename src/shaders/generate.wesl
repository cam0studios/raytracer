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
    let sizei = vec2i(vars.size);
    let sizef = vec2f(vars.size);
    // let ray = Ray();
    let coords = vec2i(id.xy);

    if coords.x >= sizei.x || coords.y >= sizei.y {
        return;
    }

    textureStore(output, coords, vec4f(cos(f32(coords.x) / sizef.x * 6.28 * 5) / 2 + 0.5, cos(f32(coords.y) / sizef.y * 6.28 * 3) / 2 + 0.5, 0, 1.0));
}