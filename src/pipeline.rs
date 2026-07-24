// wgpu compiling and encoding

/*

FOR EVERY FRAME:
    camera data  --- generate.wesl -->  ray buffer + active indices+counter
    CPU: write extend indirect buffer

    FOR EVERY BOUNCE:
        ray buffer, scene  >---------------------------------- extend.wesl --------->  intersection buffer
        CPU: clear material counters
        intersection buffer  >-------------------------------- sort.wesl ----------->  material indices+counters
        material indirect buffer, material counters  >-------- prep_indirect.wesl -->  material indirect buffer
        lambertian indices+counter, ray buffer, materials  >-- lambertian.wesl ----->  new active indices buffer
        dielectric indices+counter, ray buffer, materials  >-- dielectric.wesl ----->  new active indices buffer
        emissive indices+counter, ray buffer, materials  >---- emissive.wesl ------->  output image
        miss indices+counter, ray buffer  >------------------- miss.wesl ----------->  output image
        extend indirect buffer, active counter  >------------- prep_indirect.wesl -->  extend indirect buffer
        CPU: swap old and new active indices buffers and counters, clear used one

*/
