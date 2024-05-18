use error_iter::ErrorIter;
use log::error;
use pixels::{Pixels, SurfaceTexture};
use pixels::wgpu::Color;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{WindowEvent, DeviceEvent, DeviceId, KeyEvent, MouseButton, ElementState};
use winit::event_loop::{EventLoop, ActiveEventLoop};
use winit::keyboard::{Key, PhysicalKey};
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};
use rand::random;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;
const GRID_SIZE: usize = 200;

#[derive(Default)]
struct World {
    pixels: Option<Pixels>,
    grid: Grid
}

impl World {
    fn draw(&mut self) {
        let mut frame = self.pixels.as_mut().unwrap().frame_mut();
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let cell = self.grid.grid[i];
            // if cell.cell_type == CellType::Air {
            //     continue;
            // }

            let rgba = cell.color;
            pixel.copy_from_slice(&rgba);
        }
    }
}

struct Grid {
    grid: Box<[Cell; GRID_SIZE * GRID_SIZE]>
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            grid: Box::new([Cell::new(CellType::Air); GRID_SIZE * GRID_SIZE])
        }
    }
}

impl Grid {
    fn place(&mut self, pos: usize, cell_type: CellType) {
        let cell = &self.grid[pos];
        if cell.cell_type == CellType::Air {
            self.grid[pos] = Cell::new(cell_type);
        }
    }

    fn execute_logic(&mut self) {
        let mut i: usize = 0;
        let temp = self.grid.clone();
        for cell in temp.iter() {
            if cell.cell_type == CellType::Sand {
                if i + GRID_SIZE >= GRID_SIZE * GRID_SIZE {
                    return;
                }

                if self.grid[i + GRID_SIZE].cell_type == CellType::Air {
                    self.grid.swap(i, i + GRID_SIZE);
                    return;
                }


            }

            i += 1;
        }
    }
}

#[derive(Copy, Clone)]
struct Cell {
    cell_type: CellType,
    color: [u8; 4]
}

impl Cell {
    fn new(cell_type: CellType) -> Cell {
        let mut color:[u8;4] = [0, 0, 0, 255];

        if cell_type == CellType::Sand {
            color = [252, 186, 3, 255];
        }

        Cell {
            cell_type,
            color
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum CellType {
    Air,
    Sand
}

#[derive(Default)]
struct State {
    // Use an `Option` to allow the window to not be available until the
    // application is properly running.
    window: Option<Window>,
    window_size: LogicalSize<f64>,
    world: World,
    input: Input
}

#[derive(Default)]
struct Input {
    mouse_position: PhysicalPosition<f32>,
    left_mouse_pressed: bool
}

impl ApplicationHandler for State {
    // This is a common indicator that you can create a window.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(event_loop.create_window(
            Window::default_attributes()
                .with_title("Hello Pixels!")
                .with_inner_size(self.window_size)
                .with_min_inner_size(self.window_size)
                .with_max_inner_size(self.window_size)
        ).unwrap());

        let mut pixels = {
            let window_size = self.window.as_ref().unwrap().inner_size();
            let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, self.window.as_ref().unwrap());
            Pixels::new(WIDTH, HEIGHT, surface_texture)
        }.expect("Pixels not initialized!");
        pixels.resize_buffer(GRID_SIZE as u32, GRID_SIZE as u32).expect("Couldn't resize pixels buffer!");
        self.world.pixels = Option::from(pixels);

        println!("Resumed!");
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // `unwrap` is fine, the window will always be available when
        // receiving a window event.
        let window = self.window.as_ref().unwrap();
        match event {
            WindowEvent::RedrawRequested => {
                render(self, event_loop);
                update(self, event_loop);
            }
            WindowEvent::CloseRequested => {
                event_loop.exit()
            }
            WindowEvent::KeyboardInput { device_id, event, is_synthetic} => {
            }
            WindowEvent::CursorMoved {device_id, position} => {
                self.input.mouse_position = <(f32, f32)>::from(position).into();
            }
            WindowEvent::MouseInput {device_id, button, state} => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.input.left_mouse_pressed = true;
                        }
                        ElementState::Released => {
                            self.input.left_mouse_pressed = false;
                        }
                    }
                }
            }
            _ => ()
        }
        // Handle window event.
    }
    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        // Handle window event.
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut state = State::default();
    state.window_size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
    let _ = event_loop.run_app(&mut state);
}

fn update(state: &mut State, event_loop: &ActiveEventLoop) {
    if state.input.left_mouse_pressed {
        if state.world.pixels.is_some() {
            let pixel_pos =
                state.world.pixels.as_mut().unwrap()
                    .window_pos_to_pixel((state.input.mouse_position.x, state.input.mouse_position.y));
            if pixel_pos.is_ok() {
                let pos = pixel_pos.unwrap();
                state.world.grid.place(pos.1 * GRID_SIZE + pos.0, CellType::Sand);
            }
        }
    }

    state.world.grid.execute_logic();
}

fn render(state: &mut State, event_loop: &ActiveEventLoop) {
    state.world.draw();
    if let Err(err) = state.world.pixels.as_ref().unwrap().render() {
        log_error("pixels.render", err);
        event_loop.exit();
        return;
    }
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}