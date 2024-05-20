use error_iter::ErrorIter;
use log::error;
use pixels::{Pixels, SurfaceTexture};
use rand::{random, Rng};
use rand::prelude::ThreadRng;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{WindowEvent, DeviceEvent, DeviceId, MouseButton, ElementState};
use winit::event_loop::{EventLoop, ActiveEventLoop};
use winit::window::{Window, WindowId};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;
const GRID_WIDTH: usize = 200;
const GRID_SIZE: usize = GRID_WIDTH * GRID_WIDTH;

const CELL_AIR: CellType = CellType::Air;
const CELL_SAND: CellType = CellType::Sand([252, 186, 3, 255]);

#[derive(Default)]
struct World {
    pixels: Option<Pixels>,
    grid: Grid
}

impl World {
    fn draw(&mut self) {
        let frame = self.pixels.as_mut().unwrap().frame_mut();
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let cell = self.grid.grid[i];
            let mut rgba:&[u8;4] = &[0,0,0,255];
            match cell.cell_type {
                CellType::Sand(color) => {
                    rgba = color;
                }
                _ => {}
            }

            pixel.copy_from_slice(rgba);
        }
    }
}

struct Grid {
    grid: Vec<Cell>
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            grid: vec![Cell::new(&CELL_AIR); GRID_SIZE]
        }
    }
}

impl Grid {
    fn place(&mut self, pos: usize, cell_type: &'static CellType) {
        let cell = &self.grid[pos];
        if cell.cell_type.eq(&CELL_AIR) {
            self.grid[pos] = Cell::new(cell_type);
        }
    }

    fn place_line(&mut self, pos1: (usize, usize), pos2: (usize, usize), cell_type: &'static CellType) {
        for point in Self::generate_line(pos1, pos2){
            self.place((point.1 as usize) * GRID_WIDTH + (point.0 as usize), cell_type);
        }
    }

    fn generate_line(pos1: (usize, usize), pos2: (usize, usize)) -> Vec<(i32, i32)> {
        let mut points = vec![];

        let mut x1:i32 = pos1.0 as i32;
        let x2:i32 = pos2.0 as i32;
        let mut y1:i32 = pos1.1 as i32;
        let y2:i32 = pos2.1 as i32;

        let dx:i32 = x1.abs_diff(x2) as i32;
        let mut xpositive = true;
        if x1 > x2{
            xpositive = false;
        }
        let dy:i32 = -(y1.abs_diff(y2) as i32);
        let mut ypositive = true;
        if y1 > y2{
            ypositive = false;
        }
        let mut error = dx + dy;

        loop {
            points.append(&mut vec![(x1, y1)]);
            if x1 == x2 && y1 == y2{
                break;
            }
            let e2 = 2 * error;
            if e2 >= dy{
                if x1 == x2{
                    break;
                }
                error = error + dy;
                if xpositive {
                    x1 += 1;
                }
                else {
                    x1 -= 1;
                }
            }

            if e2 <= dx{
                if y1 == y2{
                    break
                }
                error = error + dx;
                if ypositive {
                    y1 += 1;
                }
                else {
                    y1 -= 1;
                }
            }
        }

        points
    }

    fn execute_logic(&mut self) {
        let mut changes = vec![];

        let mut i: usize = 0;
        loop {
            let mut cell: Cell = self.grid[i];
            cell.logic(self, i, &mut changes);
            self.grid[i] = cell;

            i += 1;
            if i == GRID_SIZE {
                break;
            }
        }

        for change in changes {
            self.grid.swap(change.0, change.1);
        }
    }
}

#[derive(Copy, Clone)]
struct Cell {
    cell_type: &'static CellType,
    velocity: (f32, f32),
    free_falling: bool
}

impl Cell {
    fn new(cell_type: &'static CellType) -> Cell {
        Cell {
            cell_type,
            velocity: (0.0,0.0),
            free_falling: false
        }
    }

    fn logic(&mut self, grid: &Grid, pos: usize, changes: &mut Vec<(usize, usize)>) {
        match self.cell_type {
            CellType::Sand(_) => {
                let mut rng = rand::thread_rng();

                let mut neighbours: [Option<&Cell>; 8] = [None; 8];
                let mut i = -2;
                let mut j = -2;
                let mut index = -1;
                while i < 1 {
                    i += 1;
                    let row = (pos as i32) + (i * GRID_WIDTH as i32);
                    if row < 0 || row >= GRID_SIZE as i32 {
                        index += 3;
                        continue;
                    }
                    j = -2;
                    while j < 1 {
                        j += 1;
                        let p = row + j;
                        if p == pos as i32 {
                            continue;
                        }
                        index += 1;
                        if row / GRID_WIDTH as i32 != p / GRID_WIDTH as i32 {
                            continue;
                        }

                        neighbours[index as usize] = Option::from(&grid.grid[p as usize]);
                    }
                }

                if self.velocity.0.abs() < 1.0 {
                    self.velocity.0 = 0.0;
                } else {
                    self.velocity.0 /= 1.5;
                }

                let mut grounded = false;
                if neighbours[6].is_some() {
                    if neighbours[6].unwrap().cell_type.eq(&CELL_AIR) {
                        self.velocity.1 += 0.3;
                    }
                    else { grounded = true; }
                }
                else { grounded = true; }

                if grounded {
                    let mut left_bottom_free = false;
                    if neighbours[5].is_some() {
                        if neighbours[5].unwrap().cell_type.eq(&CELL_AIR) {
                            left_bottom_free = true;
                        }
                    }

                    let mut right_bottom_free = false;
                    if neighbours[7].is_some() {
                        if neighbours[7].unwrap().cell_type.eq(&CELL_AIR) {
                            right_bottom_free = true;
                        }
                    }

                    let speed = 1.0;
                    if left_bottom_free && right_bottom_free {
                        if rng.gen_bool(0.5) {
                            right_bottom_free = false;
                        } else {
                            left_bottom_free = false;
                        }
                    }
                    if left_bottom_free {
                        self.velocity.0 -= speed;
                    } else if right_bottom_free {
                        self.velocity.0 += speed;
                    }

                    self.velocity.1 -= 0.5;
                    if self.velocity.1 < 0.0 {
                        self.velocity.1 = 0.0;
                    }
                }


                let mut new_pos = pos;
                let pos_xy = (pos % GRID_WIDTH, pos / GRID_WIDTH);
                let mut intended_pos_xy = (((pos % GRID_WIDTH) as i32) + self.velocity.0 as i32, ((pos / GRID_WIDTH) as i32) + self.velocity.1 as i32);
                if intended_pos_xy.0 < 0 {
                    intended_pos_xy.0 = 0;
                }
                if intended_pos_xy.1 < 0 {
                    intended_pos_xy.1 = 0;
                }
                if intended_pos_xy.0 >= GRID_WIDTH as i32 {
                    intended_pos_xy.0 = (GRID_WIDTH - 1) as i32;
                }
                if intended_pos_xy.1 >= GRID_WIDTH as i32 {
                    intended_pos_xy.1 = (GRID_WIDTH - 1) as i32;
                }

                let points = Grid::generate_line(pos_xy, (intended_pos_xy.0 as usize, intended_pos_xy.1 as usize));

                for point in &points[1..] {
                    let mut x = point.0;
                    let mut y = point.1;

                    let temp = (point.1 as usize) * GRID_WIDTH + point.0 as usize;
                    if grid.grid[temp].cell_type.eq(&CELL_SAND) {
                        break;
                    }

                    new_pos = (point.1 as usize) * GRID_WIDTH + point.0 as usize;
                }

                if pos != new_pos {
                    changes.append(&mut vec![(pos, new_pos)]);
                }
            }
            _ => {}
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum CellType {
    Air,
    Sand([u8;4])
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
    previous_mouse_position: PhysicalPosition<f32>,
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
        pixels.resize_buffer(GRID_WIDTH as u32, GRID_WIDTH as u32).expect("Couldn't resize pixels buffer!");
        self.world.pixels = Option::from(pixels);

        println!("Resumed!");
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // `unwrap` is fine, the window will always be available when
        // receiving a window event.
        let _window = self.window.as_ref().unwrap();
        match event {
            WindowEvent::RedrawRequested => {
                render(self, event_loop);
                update(self, event_loop);
            }
            WindowEvent::CloseRequested => {
                event_loop.exit()
            }
            WindowEvent::KeyboardInput { device_id: _device_id, event: _, is_synthetic: _} => {
            }
            WindowEvent::CursorMoved {device_id: _, position} => {
                self.input.mouse_position = <(f32, f32)>::from(position).into();
            }
            WindowEvent::MouseInput {device_id: _, button, state} => {
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
    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, _event: DeviceEvent) {
        // Handle window event.
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
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

fn update(state: &mut State, _event_loop: &ActiveEventLoop) {
    if state.input.left_mouse_pressed {
        if state.world.pixels.is_some() {
            let pixel_pos1 =
                state.world.pixels.as_mut().unwrap()
                    .window_pos_to_pixel((state.input.previous_mouse_position.x, state.input.previous_mouse_position.y));
            let pixel_pos2 =
                state.world.pixels.as_mut().unwrap()
                    .window_pos_to_pixel((state.input.mouse_position.x, state.input.mouse_position.y));
            if pixel_pos1.is_ok() && pixel_pos2.is_ok() {
                let pos1 = pixel_pos1.unwrap();
                let pos2 = pixel_pos2.unwrap();
                state.world.grid.place_line(pos1, pos2, &CELL_SAND);
            }
        }
    }

    state.input.previous_mouse_position = state.input.mouse_position;

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