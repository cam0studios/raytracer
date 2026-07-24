// wgpu setup, frame loop

// mod buffers;
// mod pipeline;
// mod scene;
mod window;

fn main() {
    // let pipeline = pipeline::Pipeline::new();
    let window_manager = window::WindowManager::new();
}

/*

NOTES:
  - use atomicAdd to get next available index for buffer pushing
  - double buffered active indices, swapping buffers every bounce
  - add atomics first in workgroups, then one write per group
  - dispatch steps with thread count equal to active counter
  - set active counter buffers as indirect to use as dispatch dimensions
  - 2 separate entry points in prep indirect
  - render pass to draw storage buffer to surface

TO DO LATER:
  - Russian roulette in lambertian/dielectric
  - Shadow rays
  - Spectral instead of RGB
  - Shader hot reloading

*/
