use glow::HasContext;

use crate::{
    math::{self, m4x4_mtranslate},
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
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
    win_width: i32,
    win_height: i32,
    viewer_pos: math::V4,
    viewer_rvec: math::V3,
    viewer_rangle: f32,
}

impl World {
    pub fn rotate_viewer(
        &mut self,
        gl: &glow::Context,
        delta_xi: f64,
        delta_eta: f64,
    ) -> Result<(), String> {
        if delta_xi == 0. && delta_eta == 0. {
            return Ok(());
        }

        let mut v = [
            delta_eta as f32 * (self.right - self.left) / self.win_height as f32,
            delta_xi as f32 * (self.top - self.bottom) / self.win_width as f32,
            0.,
        ];
        let lgt = math::v3_dot_product(&v, &v).sqrt();
        v[0] /= lgt;
        v[1] /= lgt;

        let angi = (-3.0f32).to_radians();
        let (vk, angk) = math::v3_comp_rotations(&self.viewer_rvec, self.viewer_rangle, &v, angi);
        self.viewer_rvec = vk;
        self.viewer_rangle = angk;
        let mut vm = math::m4x4_translate(
            -self.viewer_pos[0],
            -self.viewer_pos[1],
            -self.viewer_pos[2],
        );
        math::m4x4_mrotate_v(
            &mut vm,
            self.viewer_rvec[0],
            self.viewer_rvec[1],
            self.viewer_rvec[2],
            -self.viewer_rangle,
        );

        unsafe {
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.my_shaders.trbuf));
            gl.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                self.my_shaders.trbofs[1],
                bytemuck::cast_slice(&vm),
            );
            check_gl_error(gl)
        }
    }

    pub fn new(gl: &glow::Context, width: i32, height: i32) -> Result<Self, String> {
        let initial_view_pos = [0., 0., 10., 1.];
        let my_shaders = load_shaders(gl)?;
        init_model_matrix(gl, &my_shaders)?;
        init_view_matrix(gl, &my_shaders, &initial_view_pos)?;
        let icosahedron = construct_icosahedron_vao(gl)?;
        let mut world = Self {
            my_shaders,
            icosahedron,
            left: 0.,
            right: 0.,
            bottom: 0.,
            top: 0.,
            near: 0.,
            far: 0.,
            win_width: 0,
            win_height: 0,
            viewer_pos: initial_view_pos,
            viewer_rvec: [0., 0., 0.],
            viewer_rangle: 0.,
        };
        world.resize(gl, width, height)?;

        Ok(world)
    }

    pub fn resize(&mut self, gl: &glow::Context, width: i32, height: i32) -> Result<(), String> {
        let lr = 0.5533 * (width as f32) / (height as f32);
        let (m, _) = math::m4x4_frustum(-lr, lr, -0.5533, 0.5533, 5., 15.);

        self.win_width = width;
        self.win_height = height;
        self.left = -lr;
        self.right = lr;
        self.bottom = -0.5533;
        self.top = 0.5533;
        self.near = 5.;
        self.far = 15.;

        unsafe {
            gl.viewport(0, 0, width, height);
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

        utils::check_gl_error(gl)?;

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

fn init_view_matrix(
    gl: &glow::Context,
    shaders: &Shaders,
    viewer_pos: &math::V4,
) -> Result<(), String> {
    let m = math::m4x4_translate(-viewer_pos[0], -viewer_pos[1], -viewer_pos[2]);
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
