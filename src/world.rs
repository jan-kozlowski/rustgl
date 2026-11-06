use glow::HasContext;

use crate::{
    math,
    utils::{self, check_gl_error},
};

struct Shaders {
    shaders: [glow::Shader; 2],
    program: glow::Program,
    trbi: u32,
    trbsize: i32,
    trbofs: [i32; 3],
    trbuf: glow::Buffer,
}

struct Icosahedron {
    icos_vao: glow::VertexArray,
    icos_vbo: [glow::Buffer; 3],
}

pub struct World {
    my_shaders: Shaders,
    icosahedron: Icosahedron,
}

impl World {
    pub fn new(gl: &glow::Context, width: i32, height: i32) -> Result<Self, String> {
        let my_shaders = load_shaders(gl)?;
        init_model_matrix(gl, &my_shaders)?;
        init_view_matrix(gl, &my_shaders)?;
        let icosahedron = construct_icosahedron_vao(gl)?;
        let world = Self {
            my_shaders,
            icosahedron,
        };
        world.resize(gl, width, height)?;

        Ok(world)
    }

    pub fn resize(&self, gl: &glow::Context, width: i32, height: i32) -> Result<(), String> {
        unsafe {
            gl.viewport(0, 0, width, height);
        }
        let lr = 0.5533 * (width as f32) / (height as f32);
        let (m, _) = math::m4x4_frustum(-lr, lr, -0.5533, 0.5533, 5., 15.);
        unsafe {
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.my_shaders.trbuf));
            gl.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                self.my_shaders.trbofs[2],
                bytemuck::cast_slice(&m),
            );
        }
        check_gl_error(gl)
    }

    pub fn draw(&self, gl: &glow::Context) {
        unsafe {
            gl.clear_color(1., 1., 1., 1.);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.enable(glow::DEPTH_TEST);
            self.draw_icosahedron_vao(gl);
            gl.flush();
        }
    }

    fn draw_icosahedron_vao(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(Some(self.my_shaders.program));
            gl.bind_vertex_array(Some(self.icosahedron.icos_vao));

            // ściany
            gl.bind_buffer(
                glow::ELEMENT_ARRAY_BUFFER,
                Some(self.icosahedron.icos_vbo[2]),
            );
            gl.draw_elements(
                glow::TRIANGLE_FAN,
                7,
                glow::UNSIGNED_BYTE,
                36 * size_of::<u8>() as i32,
            );
            gl.draw_elements(
                glow::TRIANGLE_FAN,
                7,
                glow::UNSIGNED_BYTE,
                43 * size_of::<u8>() as i32,
            );
            gl.draw_elements(
                glow::TRIANGLE_STRIP,
                12,
                glow::UNSIGNED_BYTE,
                50 * size_of::<u8>() as i32,
            );
        }
    }

    pub fn cleanup(&self, gl: &glow::Context) -> Result<(), String> {
        unsafe {
            gl.use_program(None);
            self.my_shaders
                .shaders
                .iter()
                .for_each(|&shader_id| gl.delete_shader(shader_id));
            gl.delete_program(self.my_shaders.program);
            gl.delete_buffer(self.my_shaders.trbuf);
            gl.delete_vertex_array(self.icosahedron.icos_vao);
            self.icosahedron
                .icos_vbo
                .iter()
                .for_each(|&buffer| gl.delete_buffer(buffer));
        }
        utils::check_gl_error(gl)
    }
}

fn load_shaders(gl: &glow::Context) -> Result<Shaders, String> {
    const UTB_NAME: &str = "TransBlock";
    const UTB_MEMBERS: [&str; 3] = [
        "TransBlock.model",
        "TransBlock.view",
        "TransBlock.projection",
    ];

    let shaders = [
        utils::compile_shader_files(gl, glow::VERTEX_SHADER, &["app1.glsl.vert"])?,
        utils::compile_shader_files(gl, glow::FRAGMENT_SHADER, &["app1.glsl.frag"])?,
    ];
    let program = utils::link_shader_program(gl, &shaders)?;

    unsafe {
        let trbi = gl
            .get_uniform_block_index(program, UTB_NAME)
            .ok_or_else(|| format!("Nie znaleziono bloku uniformów `{UTB_NAME}`"))?;

        let trbsize =
            gl.get_active_uniform_block_parameter_i32(program, trbi, glow::UNIFORM_BLOCK_DATA_SIZE);

        let indices: Vec<u32> = gl
            .get_uniform_indices(program, &UTB_MEMBERS)
            .into_iter()
            .zip(UTB_MEMBERS)
            .map(|(idx, name)| idx.ok_or_else(|| format!("Nie znaleziono uniformu `{name}`")))
            .collect::<Result<_, _>>()?;

        let trbofs: [i32; 3] = gl
            .get_active_uniforms_parameter(program, &indices, glow::UNIFORM_OFFSET)
            .try_into()
            .map_err(|v: Vec<i32>| format!("Oczekiwano 3 offsetów, otrzymano {}", v.len()))?;

        gl.uniform_block_binding(program, trbi, 0);

        let trbuf = gl.create_buffer()?;
        gl.bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(trbuf));
        gl.buffer_data_size(glow::UNIFORM_BUFFER, trbsize, glow::DYNAMIC_DRAW);

        utils::check_gl_error(gl);

        Ok(Shaders {
            shaders,
            program,
            trbi,
            trbsize,
            trbofs,
            trbuf,
        })
    }
}

fn init_model_matrix(gl: &glow::Context, shaders: &Shaders) -> Result<(), String> {
    let m = math::m4x4_ident();
    unsafe {
        gl.bind_buffer(glow::UNIFORM_BUFFER, Some(shaders.trbuf));
        gl.buffer_sub_data_u8_slice(
            glow::UNIFORM_BUFFER,
            shaders.trbofs[0],
            bytemuck::cast_slice(&m),
        );
    }
    utils::check_gl_error(gl)
}

fn init_view_matrix(gl: &glow::Context, shaders: &Shaders) -> Result<(), String> {
    let m = math::m4x4_translate(0., 0., -10.);
    unsafe {
        gl.bind_buffer(glow::UNIFORM_BUFFER, Some(shaders.trbuf));
        gl.buffer_sub_data_u8_slice(
            glow::UNIFORM_BUFFER,
            shaders.trbofs[1],
            bytemuck::cast_slice(&m),
        );
    }
    utils::check_gl_error(gl)
}

fn construct_icosahedron_vao(gl: &glow::Context) -> Result<Icosahedron, String> {
    const A: f32 = 0.52573115;
    const B: f32 = 0.85065085;
    let vertpos: [[f32; 3]; 12] = [
        [-A, 0.0, -B],
        [A, 0.0, -B],
        [0.0, -B, -A],
        [-B, -A, 0.0],
        [-B, A, 0.0],
        [0.0, B, -A],
        [A, 0.0, B],
        [-A, 0.0, B],
        [0.0, -B, A],
        [B, -A, 0.0],
        [B, A, 0.0],
        [0.0, B, A],
    ];
    let vertcol: [[u8; 3]; 12] = [
        [255, 0, 0],
        [255, 127, 0],
        [255, 255, 0],
        [127, 255, 0],
        [0, 255, 0],
        [0, 255, 127],
        [0, 255, 255],
        [0, 127, 255],
        [0, 0, 255],
        [127, 0, 255],
        [255, 0, 255],
        [255, 0, 127],
    ];
    let vertind: [u8; 62] = [
        0, 1, 2, 0, 3, 4, 0, 5, 1, 9, 2, 8, 3, /* lamana, od 0 */
        7, 4, 11, 5, 10, 9, 6, 8, 7, 6, 11, 7, 1, 10, 6, /* lamana, od 25 */
        2, 3, 4, 5, 8, 9, 10, 11, /* 4 odcinki, od 28 */
        0, 1, 2, 3, 4, 5, 1, /* wachlarz, od 36 */
        6, 7, 8, 9, 10, 11, 7, /* wachlarz, od 43  */
        1, 9, 2, 8, 3, 7, 4, 11, 5, 10, 1, 9,
    ]; /* tasma, od 50 */

    unsafe {
        let icos_vao = gl.create_vertex_array()?;
        gl.bind_vertex_array(Some(icos_vao));

        let icos_vbo = [
            gl.create_buffer().unwrap(),
            gl.create_buffer().unwrap(),
            gl.create_buffer().unwrap(),
        ];
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(icos_vbo[0]));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertpos),
            glow::STATIC_DRAW,
        );
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 3 * size_of::<f32>() as i32, 0);

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(icos_vbo[1]));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertcol),
            glow::STATIC_DRAW,
        );
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(
            1,
            3,
            glow::UNSIGNED_BYTE,
            true,
            (3 * size_of::<u8>()) as i32,
            0,
        );

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(icos_vbo[2]));
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &vertind, glow::STATIC_DRAW);

        utils::check_gl_error(gl)?;
        Ok(Icosahedron { icos_vao, icos_vbo })
    }
}
