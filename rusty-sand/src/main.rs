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
const CELL_SAND: CellType = CellType::Sand;
const CELL_DIRT: CellType = CellType::Dirt;

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
            let rgba:&[u8;4] = &cell.color;

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
        for point in generate_line(pos1, pos2){
            self.place((point.1 as usize) * GRID_WIDTH + (point.0 as usize), cell_type);
        }
    }

    fn execute_logic(&mut self) {
        let mut changes = Changes::default();

        for i in 0..GRID_SIZE {
            let mut cell: Cell = self.grid[i];
            cell.logic(self, i, &mut changes);
            self.grid[i] = cell;
        }

        for pos in changes.pos {
            self.grid.swap(pos.0, pos.1);
        }
        for free_falling in changes.free_falling {
            self.grid[free_falling.0].free_falling = free_falling.1;
        }
    }
}

#[derive(Default)]
struct Changes {
    pos: Vec<(usize, usize)>,
    free_falling: Vec<(usize, u8)>
}

#[derive(Copy, Clone)]
struct Cell {
    cell_type: &'static CellType,
    velocity: (f32, f32),
    free_falling: u8,
    pos: usize,
    grounded: bool,
    color: [u8;4]
}

impl Cell {
    fn new(cell_type: &'static CellType) -> Cell {
        let mut color = [0u8;4];
        match cell_type {
            CellType::Sand => {
                color = [252, 186, 3, 255];
            }
            CellType::Dirt => {
                color = [89, 44, 20, 255];
            }
            _ => {
                color = [0,0,0, 255];
            }
        }
        Cell {
            cell_type,
            velocity: (0.0,0.0),
            free_falling: 0,
            pos: GRID_SIZE + 1,
            grounded: false,
            color
        }
    }

    fn logic(&mut self, grid: &Grid, pos: usize, changes: &mut Changes) {
        match self.cell_type {
            CellType::Sand => {
                self.movable_solid_logic(grid, pos, changes); // Calculate all the forces and set them to self.velocity

                let mut new_pos = pos;
                if self.velocity.0 != 0.0 || self.free_falling < 4 {
                    new_pos = self.physics(grid, pos); // Calculate physics based on the self.velocity
                }

                if pos != new_pos {
                    changes.pos.append(&mut vec![(pos, new_pos)]);
                }
                self.pos = pos;
            }
            CellType::Dirt => {
                self.movable_solid_logic(grid, pos, changes); // Calculate all the forces and set them to self.velocity

                let mut new_pos = pos;
                if self.velocity.0 != 0.0 || self.free_falling < 4 {
                    new_pos = self.physics(grid, pos); // Calculate physics based on the self.velocity
                }

                if pos != new_pos {
                    changes.pos.append(&mut vec![(pos, new_pos)]);
                }
                self.pos = pos;
            }
            _ => {}
        }
    }

    fn movable_solid_logic(&mut self, grid: &Grid, pos: usize, changes: &mut Changes) {
        let mut rng = rand::thread_rng();
        let free_falling_threshold = 4u8;
        let mut neighbours: [(usize, Option<&Cell>);8] = [(0, None); 8];

        // Set the free-falling flag of neighbour cells
        if self.free_falling < free_falling_threshold {
            neighbours = Self::get_neighbours(grid, pos);
            let mut movable_solid_neighbours = vec![];
            for (p, n) in neighbours {
                if n.is_some() {
                    if CellType::is_movable_solid(n.unwrap().cell_type) {
                        movable_solid_neighbours.append(&mut vec![p])
                    }
                }
            }

            if movable_solid_neighbours.len() < 5 {
                for n in movable_solid_neighbours {
                    if rng.gen_bool(1.0 - CellType::get_inertial_resistance(grid.grid[n].cell_type)) {
                        changes.free_falling.append(&mut vec![(n, free_falling_threshold * 2)]);
                    }
                }
            }
        }
        else {
            if self.free_falling == free_falling_threshold * 2 {
                for i in 0..3 {
                    neighbours[5 + i] = Self::get_neighbour(grid, pos, (-1 + i as i8, 1))
                }
            }
            else {
                neighbours[6] = Self::get_neighbour(grid, pos, (0, 1));
            }
        }


        // Validate the change of free-falling flag by external cell
        if self.free_falling == free_falling_threshold * 2 {
            let mut occupied_count: u8 = 0;
            for i in 0..3 {
                if neighbours[5 + i].1.is_some() {
                    if CellType::is_solid(neighbours[5 + i].1.unwrap().cell_type) {
                        occupied_count += 1;
                    }
                }
                else {
                    occupied_count += 1;
                }
            }

            if occupied_count == 3 {
                self.free_falling = free_falling_threshold;
            }
            else {
                self.free_falling = 0;
            }
        }


        // Has solid under
        let mut grounded = true;
        if neighbours[6].1.is_some() {
            if neighbours[6].1.unwrap().cell_type.eq(&CELL_AIR) {
                grounded = false;
            }
        }

        // Horizontal velocity drag
        if self.velocity.0.abs() >= 1.0 {
            self.velocity.0 *= 0.8;
            if self.velocity.0.abs() <= 1.0 {
                self.velocity.0 = 0.0;
            }
        }

        if !grounded {
            self.velocity.1 += 0.3; // Gravity
            self.free_falling = 0;
        }
        else {
            if !self.grounded { // If grounded from previous frame was false - did it just hit the ground
                let r:f64 = rng.gen();
                let absorbed_speed = 4.0_f32.min(self.velocity.1 * (r as f32));

                let mut left_free = false;
                if neighbours[3].1.is_some() && self.velocity.0 <= 0.0 {
                    if neighbours[3].1.unwrap().cell_type.eq(&CELL_AIR) {
                        left_free = true;
                    }
                }
                let mut right_free = false;
                if neighbours[4].1.is_some() && self.velocity.0 >= 0.0 {
                    if neighbours[4].1.unwrap().cell_type.eq(&CELL_AIR) {
                        right_free = true;
                    }
                }

                if left_free && right_free {
                    if rng.gen_bool(0.5) {
                        left_free = false;
                    } else {
                        right_free = false;
                    }
                }

                if left_free {
                    self.velocity.0 = -absorbed_speed;
                } else if right_free {
                    self.velocity.0 = absorbed_speed;
                }
            }

            else if self.free_falling < free_falling_threshold { // Is in free-fall state
                let mut left_bottom_free = false;
                if neighbours[5].1.is_some() && self.velocity.0 <= 0.0 {
                    if neighbours[5].1.unwrap().cell_type.eq(&CELL_AIR) {
                        left_bottom_free = true;
                    }
                }
                let mut right_bottom_free = false;
                if neighbours[7].1.is_some() && self.velocity.0 >= 0.0 {
                    if neighbours[7].1.unwrap().cell_type.eq(&CELL_AIR) {
                        right_bottom_free = true;
                    }
                }

                if left_bottom_free && right_bottom_free {
                    if rng.gen_bool(0.5) {
                        left_bottom_free = false;
                    } else {
                        right_bottom_free = false;
                    }
                }
                else if rng.gen_bool(CellType::get_inertial_resistance(&CELL_SAND).powf(3.0)) { // There is a chance for the cell to stop moving at the edge of the hill
                    self.free_falling = free_falling_threshold;
                    left_bottom_free = false;
                    right_bottom_free = false;
                }

                if left_bottom_free {
                    self.velocity.0 = -CellType::get_roll_speed(self.cell_type);
                } else if right_bottom_free {
                    self.velocity.0 = CellType::get_roll_speed(self.cell_type);
                }
            }

            self.velocity.1 = CellType::get_roll_speed(self.cell_type); // Constant weight kinda
            if self.pos == pos { // If the pos didn't change from the previous frame
                self.free_falling += 1;
                if self.free_falling > free_falling_threshold {
                    self.free_falling = free_falling_threshold;
                    self.velocity.0 = 0.0;
                }
            }
        }

        self.grounded = grounded;
    }

    fn physics(&mut self, grid: &Grid, pos: usize) -> usize {
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

        let steps = line_to_steps(&generate_line(pos_xy, (intended_pos_xy.0 as usize, intended_pos_xy.1 as usize)));

        let mut new_point = (pos_xy.0 as i32, pos_xy.1 as i32);
        for step in &steps {
            let mut point_xy = (new_point.0 + step.0, new_point.1 + step.1);
            let mut temp = (point_xy.1 as usize) * GRID_WIDTH + point_xy.0 as usize;
            if CellType::is_solid(grid.grid[temp].cell_type) {
                if step.0 == 0 || step.1 == 0 {
                    break;
                }

                let mut temp_xy = (point_xy.0, new_point.1);
                temp = (temp_xy.1 as usize) * GRID_WIDTH + temp_xy.0 as usize;
                if CellType::is_solid(grid.grid[temp].cell_type) {
                    temp_xy = (new_point.0, point_xy.1);
                    temp = (temp_xy.1 as usize) * GRID_WIDTH + temp_xy.0 as usize;
                    if CellType::is_solid(grid.grid[temp].cell_type) {
                        break;
                    }
                }

                point_xy = temp_xy;
            }
            new_point = point_xy;
            new_pos = (new_point.1 as usize) * GRID_WIDTH + new_point.0 as usize;
        }

        new_pos
    }

    fn get_neighbours(grid: &Grid, pos: usize) -> [(usize, Option<&Cell>);8] {
        let mut neighbours: [(usize, Option<&Cell>);8] = [(0, None); 8];
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
                if row / GRID_WIDTH as i32 != p / GRID_WIDTH as i32 || p < 0 || p >= GRID_SIZE as i32 {
                    continue;
                }

                neighbours[index as usize] = (p as usize ,Option::from(&grid.grid[p as usize]));
            }
        }

        neighbours
    }

    fn get_neighbour(grid: &Grid, pos: usize, dir: (i8, i8)) -> (usize, Option<&Cell>) {
        let mut neighbour = (0, None);

        let x: i32 = ((pos % GRID_WIDTH) as i32) + dir.0 as i32;
        let y: i32 = ((pos / GRID_WIDTH) as i32) + dir.1 as i32;

        if x < 0 || x >= GRID_WIDTH as i32 || y < 0 || y >= GRID_WIDTH as i32 {
            return neighbour
        }

        let p = (y as usize) * GRID_WIDTH + x as usize;
        neighbour = (p, Option::from(&grid.grid[p]));

        neighbour
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum CellType {
    Air,
    Sand,
    Stone,
    Water,
    Dirt,
    Coal,
    Co2
}

impl CellType {
    fn is_solid(cell_type: &CellType) -> bool {
        match cell_type {
            CellType::Air => { false }
            CellType::Sand => { true }
            CellType::Stone => { true }
            CellType::Water => { false }
            CellType::Dirt => { true }
            CellType::Coal => { true }
            CellType::Co2 => { false }
        }
    }

    fn is_movable_solid(cell_type: &CellType) -> bool {
        match cell_type {
            CellType::Air => { false }
            CellType::Sand => { true }
            CellType::Stone => { false }
            CellType::Water => { false }
            CellType::Dirt => { true }
            CellType::Coal => { true }
            CellType::Co2 => { false }
        }
    }

    fn get_inertial_resistance(cell_type: &CellType) -> f64 {
        match cell_type {
            CellType::Sand => { 0.1 }
            CellType::Dirt => { 0.4 }
            CellType::Coal => { 0.8 }
            _ => { 0.0 }
        }
    }

    fn get_roll_speed(cell_type: &CellType) -> f32 {
        match cell_type {
            CellType::Air => { 0.0 }
            CellType::Sand => { 2.0 }
            CellType::Dirt => { 1.5 }
            CellType::Coal => { 1.0 }
            _ => { 0.0 }
        }
    }
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
    left_mouse_pressed: bool,
    right_mouse_pressed: bool
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
                else if button == MouseButton::Right {
                    match state {
                        ElementState::Pressed => {
                            self.input.right_mouse_pressed = true;
                        }
                        ElementState::Released => {
                            self.input.right_mouse_pressed = false;
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
    if state.input.left_mouse_pressed || state.input.right_mouse_pressed {
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
                if state.input.left_mouse_pressed {
                    state.world.grid.place_line(pos1, pos2, &CELL_SAND);
                }
                else {
                    state.world.grid.place_line(pos1, pos2, &CELL_DIRT);
                }
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
fn line_to_steps(line: &Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut steps = vec![];

    let mut previous_point = line[0];
    for point in &line[1..] {
        steps.append(&mut vec![(point.0 - previous_point.0, point.1 - previous_point.1)]);
        previous_point = point.clone();
    }

    steps
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}