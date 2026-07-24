// wgpu setup, window init, frame loop

fn main() {
    println!("Hello, world!");
}

/*

NOTES:
  - use atomicAdd to get next available index for buffer pushing
  - double buffered active indices, swapping buffers every bounce
  - add atomics first in workgroups, then one write per group
  - dispatch steps with thread count equal to active counter
  - set active counter buffers as indirect to use as dispatch dimensions
  - 2 separate entry points in prep indirect

TO DO LATER:
  - Russian roulette in lambertian/dielectric
  - Shadow rays

*/
