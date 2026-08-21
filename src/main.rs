// init

mod buffers;
mod pipeline;
mod scene;
mod window;

fn main() {
    env_logger::init();
    let _window_manager = window::WindowManager::new();
}

/*

NOTES:
  - use atomicAdd to get next available index for buffer pushing
  - add atomics first in workgroups, then one write per group
  - set active counter buffers as indirect to use as dispatch dimensions
  - render pass to draw storage buffer to surface

TO FIX:
  - horizontal brightness line in render
  - max size of 2^22 px

TO DO LATER:
  - BRDFs + importance sampling
  - russian roulette in materials
  - shadow rays
  - spectral instead of RGB (each ray has random wavelength instead of rgb)
  - array of structs vs struct of arrays?
  - constant number of rays that regenerate when terminated, no fixed frame borders?
  - shader hot reloading?
  - OIDN denoising?

*/
