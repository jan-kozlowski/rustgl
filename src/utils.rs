use glow::HasContext;

pub fn print_gl_version(gl: &glow::Context) {
    unsafe {
        println!(
            "OpenGL: {}, GLSL: {}",
            gl.get_parameter_string(glow::VERSION),
            gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION)
        )
    }
}

fn gl_error_name(err: u32) -> &'static str {
    match err {
        glow::INVALID_ENUM => "GL_INVALID_ENUM",
        glow::INVALID_VALUE => "GL_INVALID_VALUE",
        glow::INVALID_OPERATION => "GL_INVALID_OPERATION",
        glow::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
        glow::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
        glow::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
        glow::STACK_OVERFLOW => "GL_STACK_OVERFLOW",
        _ => "nieznany błąd",
    }
}

pub fn check_gl_error(gl: &glow::Context) -> Result<(), String> {
    let error = unsafe { gl.get_error() };
    if error != glow::NO_ERROR {
        return Err(format!("Error {error:#06X}: {}", gl_error_name(error)));
    }
    Ok(())
}

pub fn compile_shader_strings(
    gl: &glow::Context,
    shader_type: u32,
    sources: &[&str],
) -> Result<glow::Shader, String> {
    unsafe {
        let shader = gl.create_shader(shader_type)?;
        gl.shader_source(shader, &sources.concat());
        gl.compile_shader(shader);

        let log = gl.get_shader_info_log(shader);
        if !log.trim().is_empty() {
            eprintln!("{log}");
        }
        if !gl.get_shader_compile_status(shader) {
            gl.delete_shader(shader);
            return Err(format!("Shader compilation failed: {log}"));
        }
        check_gl_error(gl)?;
        Ok(shader)
    }
}

pub fn compile_shader_files(
    gl: &glow::Context,
    shader_type: u32,
    files: &[&str],
) -> Result<glow::Shader, String> {
    let sources = files
        .iter()
        .map(|&f| std::fs::read_to_string(f).map_err(|e| format!("Could not read `{f}`: {e}")))
        .collect::<Result<Vec<String>, String>>()?;
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    compile_shader_strings(gl, shader_type, &sources)
}

pub fn link_shader_program(
    gl: &glow::Context,
    shaders: &[glow::Shader],
) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;
        for &shader in shaders {
            gl.attach_shader(program, shader);
        }
        gl.link_program(program);

        let log = gl.get_program_info_log(program);
        if !log.trim().is_empty() {
            eprintln!("{log}");
        }
        if !gl.get_program_link_status(program) {
            gl.delete_program(program);
            return Err(format!("Linking failed: {log}"));
        }
        check_gl_error(gl)?;
        Ok(program)
    }
}
