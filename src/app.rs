use std::{cell::RefCell, rc::Rc};

use glfw::{Action, Context, Glfw, Key, Modifiers, MouseButton, PWindow, Scancode, Window};

use crate::{utils, world::World};

const WIDTH: u32 = 480;
const HEIGHT: u32 = 360;

struct State {
    gl: glow::Context,
    world: World,
    redraw: bool,
    rotating: bool,
    error: Option<String>,
    last_x: f64,
    last_y: f64,
}

type SharedState = Rc<RefCell<State>>;

struct App {
    glfw: Glfw,
    window: PWindow,
    state: Rc<RefCell<State>>,
}

impl App {
    fn new() -> Result<Self, String> {
        let mut glfw = glfw::init(glfw_error_handler).expect("glfwInit failed");
        glfw.window_hint(glfw::WindowHint::ContextVersion(4, 2));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(
            glfw::OpenGlProfileHint::Core,
        ));

        let (mut window, events) = glfw
            .create_window(WIDTH, HEIGHT, "GLFW app", glfw::WindowMode::Windowed)
            .ok_or("Could not create a window")?;
        window.make_current();
        let (width, height) = window.get_framebuffer_size();

        let gl = unsafe {
            glow::Context::from_loader_function(|procname| {
                window
                    .get_proc_address(procname)
                    .map_or(std::ptr::null(), |f| f as *const _)
            })
        };
        utils::print_gl_version(&gl);

        let my_world = World::new(&gl, width, height)?;

        let state = Rc::new(RefCell::new(State {
            gl,
            world: my_world,
            redraw: true,
            rotating: false,
            error: None,
            last_x: 0.,
            last_y: 0.,
        }));

        let s = Rc::clone(&state);
        window.set_framebuffer_size_callback(move |w, width, height| {
            reshape_func(&s, w, width, height);
        });
        window.set_framebuffer_size_polling(true);

        let s = Rc::clone(&state);
        window.set_refresh_callback(move |w| {
            display_func(&s, w);
        });
        window.set_refresh_polling(true);

        let s = Rc::clone(&state);
        window.set_key_callback(move |w, key, scancode, action, modifiers| {
            keyboard_func(&s, w, key, scancode, action, modifiers);
        });
        window.set_key_polling(true);

        let s = Rc::clone(&state);
        window.set_char_callback(move |w, c| {
            char_func(w, c);
        });
        window.set_char_polling(true);

        let s = Rc::clone(&state);
        window.set_mouse_button_callback(move |w, mouse_button, action, modifiers| {
            mouse_func(&s, w, mouse_button, action, modifiers);
        });

        let s = Rc::clone(&state);
        window.set_cursor_pos_callback(move |w, x, y| {
            motion_func(&s, w, x, y);
        });

        Ok(Self {
            glfw,
            window,
            state,
        })
    }

    fn message_loop(&mut self) -> Result<(), String> {
        while !self.window.should_close() {
            {
                let mut state = self.state.borrow_mut();
                if let Some(error) = state.error.take() {
                    return Err(error);
                }
                if state.redraw {
                    state.world.draw(&state.gl);
                    state.redraw = false;
                    self.window.swap_buffers();
                }
            }
            self.glfw.wait_events();
        }

        match self.state.borrow_mut().error.take() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let state = self.state.borrow();
        if let Err(error) = state.world.cleanup(&state.gl) {
            eprintln!("Error while cleaning up: {}", error);
        }
    }
}

fn glfw_error_handler(error: glfw::Error, description: String) {
    eprintln!("GLFW error {error:?}: {description}");
}

fn report(st: &mut State, window: &mut Window, result: Result<(), String>) {
    if let Err(e) = result {
        st.error.get_or_insert(e);
        window.set_should_close(true);
    }
}

fn reshape_func(state: &SharedState, window: &mut Window, width: i32, height: i32) {
    let mut state_ref = state.borrow_mut();
    let state = &mut *state_ref;
    let result = state.world.resize(&state.gl, width, height);
    report(state, window, result);
    state.redraw = true;
}

fn keyboard_func(
    state: &SharedState,
    window: &mut Window,
    key: Key,
    scancode: Scancode,
    action: Action,
    modifiers: Modifiers,
) {
    if key == Key::Escape && action == Action::Press {
        window.set_should_close(true);
    }
}

fn char_func(window: &mut Window, c: char) {}

fn mouse_func(
    state: &SharedState,
    window: &mut Window,
    mouse_button: MouseButton,
    action: Action,
    modifiers: Modifiers,
) {
    if mouse_button != MouseButton::Button1 {
        return;
    }

    let mut state_ref = state.borrow_mut();
    let state = &mut *state_ref;

    match action {
        Action::Release => {
            state.rotating = false;
        }
        Action::Press => {
            let (x, y) = window.get_cursor_pos();
            state.rotating = true;
            state.last_x = x;
            state.last_y = y;
        }
        Action::Repeat => {}
    }
}

fn motion_func(state: &SharedState, window: &mut Window, x: f64, y: f64) {
    let mut state_ref = state.borrow_mut();
    let state = &mut *state_ref;

    if !state.rotating {
        return;
    }

    let delta_x = x - state.last_x;
    let delta_y = y - state.last_y;
    if delta_x != 0. || delta_y != 0. {
        let result = state.world.rotate_viewer(&state.gl, delta_x, delta_y);
        report(state, window, result);

        state.last_x = x;
        state.last_y = y;
        state.redraw = true;
    }
}

fn display_func(state: &SharedState, window: &mut Window) {
    state.borrow_mut().redraw = true;
}

pub fn run() -> Result<(), String> {
    let mut app = App::new()?;
    app.message_loop()?;
    Ok(())
}
