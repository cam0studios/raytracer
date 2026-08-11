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

TO FIX:
  - max size of 2^22 px

TO DO LATER:
  - BRDFs + importance sampling
  - Russian roulette in materials
  - Constant number of rays that regenerate when terminated, no fixed frame borders
  - Shadow rays
  - Spectral instead of RGB (each ray has random wavelength instead of rgb)
  - Shader hot reloading?
  - OIDN denoising?

*/
