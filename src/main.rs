// init

mod buffers;
mod pipeline;
mod scene;
mod window;

fn main() {
    env_logger::init();
    scene::test();
    let _window_manager = window::WindowManager::new();
}

/*

NOTES:
  - use atomicAdd to get next available index for buffer pushing
  - add atomics first in workgroups, then one write per group
  - set active counter buffers as indirect to use as dispatch dimensions
  - render pass to draw storage buffer to surface

TO DO LATER:
  - BRDFs
  - Russian roulette in lambertian/dielectric
  - Shadow rays
  - Spectral instead of RGB (each ray has random wavelength instead of rgb)
  - Shader hot reloading
  - OIDN denoising

*/
