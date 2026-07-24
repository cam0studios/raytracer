// wgpu compiling and encoding
use wesl::{StandardResolver, Wesl};

pub struct Pipeline {
    pub generate_pipeline: wgpu::ComputePipeline,
    pub extend_pipeline: wgpu::ComputePipeline,
    pub sort_pipeline: wgpu::ComputePipeline,
    pub prep_indirect_material_pipeline: wgpu::ComputePipeline,
    pub lambertian_pipeline: wgpu::ComputePipeline,
    pub dielectric_pipeline: wgpu::ComputePipeline,
    pub emissive_pipeline: wgpu::ComputePipeline,
    pub miss_pipeline: wgpu::ComputePipeline,
    pub prep_indirect_extend_pipeline: wgpu::ComputePipeline,
    pub blit_pipeline: wgpu::RenderPipeline,
}
impl Pipeline {
    pub fn new(/*device: &wgpu::Device*/) -> Self {
        let compiler = Wesl::new("src/shaders");
        let generate_shader_string = Self::get_shader_string(&compiler, "generate");
        let extend_shader_string = Self::get_shader_string(&compiler, "extend");
        let sort_shader_string = Self::get_shader_string(&compiler, "sort");
        let prep_indirect_shader_string = Self::get_shader_string(&compiler, "prep_indirect");
        let lambertian_shader_string = Self::get_shader_string(&compiler, "lambertian");
        let dielectric_shader_string = Self::get_shader_string(&compiler, "dielectric");
        let emissive_shader_string = Self::get_shader_string(&compiler, "emissive");
        let miss_shader_string = Self::get_shader_string(&compiler, "miss");
        let blit_shader_string = Self::get_shader_string(&compiler, "blit");
        Self {}
    }

    fn get_shader_string(compiler: &Wesl<StandardResolver>, name: &str) -> String {
        compiler
            .compile(&("package::".to_string() + name).parse().unwrap())
            .inspect_err(|e| eprintln!("WESL error: {e}"))
            .unwrap()
            .to_string()
    }
}

/*

CPU: write scene data

FOR EVERY FRAME:
    camera data  --- generate.wesl -->  ray buffer + active indices+counter
    CPU: write extend indirect buffer

    FOR EVERY BOUNCE:
        ray buffer, scene  >---------------------------------- extend.wesl --------->  intersection buffer
        CPU: clear material counters
        intersection buffer  >-------------------------------- sort.wesl ----------->  material indices+counters
        material indirect buffer  >--------------------------- prep_indirect.wesl -->  material indirect buffer
        lambertian indices+counter, ray buffer, materials  >-- lambertian.wesl ----->  new active indices buffer
        dielectric indices+counter, ray buffer, materials  >-- dielectric.wesl ----->  new active indices buffer
        emissive indices+counter, ray buffer, materials  >---- emissive.wesl ------->  output buffer
        miss indices+counter, ray buffer  >------------------- miss.wesl ----------->  output buffer
        extend indirect buffer  >----------------------------- prep_indirect.wesl -->  extend indirect buffer
        CPU: swap old and new active indices buffers and counters, clear used one

    output buffer, frame count  --- blit.wesl -->  output image

*/
