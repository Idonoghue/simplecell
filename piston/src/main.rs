extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::{EventSettings, Events};
use piston::input::{Button, PressEvent, RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use piston::{MouseCursorEvent, MouseScrollEvent, ReleaseEvent};

#[derive(Clone, Copy)]
struct Coordinates {
    x: f64,
    y: f64,
}

#[derive(Debug)]
struct Simulation {
    ruleset: u8,
    steps_simulated: usize,
    cells: Vec<bool>,
}

impl Simulation {
    fn from(ruleset: u8, steps: Option<usize>) -> Simulation {
        let mut simulation = Simulation {
            ruleset,
            steps_simulated: 1,
            cells: vec![true; 1],
        };

        let steps = steps.unwrap_or(1);

        // if initial steps to simulate it more than 1 simulate steps
        if steps > 1 {
            simulation.simulate_steps(steps);
        }

        simulation
    }
    fn simulate_steps(&mut self, steps: usize) {
        // one dimensional simulation can be stored in a 1d array of length steps^2
        self.cells
            .resize((self.steps_simulated + steps).pow(2), false);

        println!("{}", &self.ruleset);

        for row_index in self.steps_simulated..self.steps_simulated + steps {
            // each row length of the expanding triangle equal to twice the height plus 1
            let row_length = (row_index * 2) + 1;
            // previous row length needed when finding the index of the parents
            let prev_row_length = row_length - 2;

            let row_start_cell_index = row_index.pow(2);

            for index in 0..row_length {
                println!(
                    "row: {}, index: {}, row_length: {}",
                    row_index, index, row_length
                );

                let left_parent: bool = match index {
                    // if cell is on the first 2 indexes of row left parent will be outside triangle
                    0 | 1 => false,

                    _ => {
                        self.cells[(row_start_cell_index + (index - 2) - prev_row_length) as usize]
                    }
                };

                let top_parent: bool = match index {
                    // if cell is on either the first or last index of row top parent will be outside triangle
                    0 => false,
                    _ if index == row_length - 1 => false,

                    _ => {
                        self.cells[(row_start_cell_index + (index - 1) - prev_row_length) as usize]
                    }
                };

                let right_parent: bool = match index {
                    // if cell is on last 2 positions on right side of row right parent will be outside triangle
                    _ if index >= row_length - 2 => false,

                    _ => self.cells[(row_start_cell_index + index - prev_row_length) as usize],
                };

                // create integer where last 3 digits represent parent states
                let parent_pattern = right_parent as u8
                    + if top_parent { 2 } else { 0 }
                    + if left_parent { 4 } else { 0 };

                println!("{}  {}  {}", left_parent, top_parent, right_parent);
                println!("parent:     {}", parent_pattern);
                println!(
                    "cell index:     {}",
                    (row_start_cell_index + index) as usize
                );
                println!(
                    "ruleset offset: {}
                
                ",
                    self.ruleset >> parent_pattern
                );

                // shift bits of ruleset number by the amount specified by parentPattern to get the right rule for the parentPattern
                self.cells[(row_start_cell_index + index) as usize] =
                    (self.ruleset >> parent_pattern) % 2 != 0
            }
        }
        self.steps_simulated += steps;
    }
}

fn main() {
    // create a CA simulation based on a ruleset
    let simulation = Simulation::from(235, None);

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // create an Glutin window.
    let mut window: Window = WindowSettings::new("Elementary Cellular Automata", [700, 700])
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    // initialize the OpenGL app
    let mut app = App {
        gl: GlGraphics::new(opengl),
        simulation,
        global_transform: Coordinates { x: 0.0, y: 0.0 },
        zoom: 1.0,
        delta_ts: [0.0; 10],
    };

    // initial cursor state
    let mut prev_cursor_pos = [0.0, 0.0];
    let mut cursor_pos = [0.0, 0.0];
    let mut cursor_down = false;

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        // render loop
        if let Some(args) = e.render_args() {
            app.render(&args);
        }

        if let Some(args) = e.update_args() {
            app.update(&args);
        }

        // update global transform if the cursor is held down
        if cursor_down {
            app.global_transform.x += cursor_pos[0] - prev_cursor_pos[0];
            app.global_transform.y += cursor_pos[1] - prev_cursor_pos[1];
            prev_cursor_pos = cursor_pos;
        }

        // when mouse pressed capture position
        if let Some(Button::Mouse(_button)) = e.press_args() {
            cursor_down = true;
            prev_cursor_pos = cursor_pos;
        }

        // when mouse is released update the global transform
        if let Some(Button::Mouse(_button)) = e.release_args() {
            cursor_down = false;
        }

        // when mouse is released update the global transform
        if let Some(Button::Mouse(_button)) = e.release_args() {
            cursor_down = false;
        }

        // update zoom
        e.mouse_scroll(|d| {
            app.zoom += if d[1].is_sign_positive() {
                0.008 * d[1]
            } else {
                app.zoom / 80.0 * d[1]
            };
            println!("Scrolled mouse '{}, {}, zoom: {}'", d[0], d[1], app.zoom)
        });

        // update cursor position
        e.mouse_cursor(|pos| {
            cursor_pos = pos;
        });
    }
}

pub struct App {
    gl: GlGraphics,                // OpenGL drawing backend.
    simulation: Simulation,        // Cellular Automata simulation
    global_transform: Coordinates, // current position of 'camera'
    zoom: f64,                     // zoom dictates how large the squares appear
    delta_ts: [f64; 10],           // Vec containing most recent delta t's to calculate fps
}

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

const DEFAULT_SQUARE_SIZE: f64 = 5.0;

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let square_size = DEFAULT_SQUARE_SIZE * self.zoom;

        // offset all positions to center the middle line horizontally on screen
        let square = rectangle::square(args.window_size[0] / 2.0, 0.0, square_size);

        let first_visible_row = (-self.global_transform.y / square_size).ceil() as usize;

        let last_visible_row: usize =
            first_visible_row + (args.window_size[1] / square_size).ceil() as usize;

        // simulate new rows if close enough to position of 'camera'
        if last_visible_row > self.simulation.steps_simulated {
            self.simulation
                .simulate_steps(last_visible_row - self.simulation.steps_simulated)
        }

        let cell_arr = &self.simulation.cells;
        let global_transform = &self.global_transform;

        self.gl.draw(args.viewport(), |c, gl| {
            // clear the screen
            clear(WHITE, gl);

            // let num_of_rows = (cell_arr.len() as f64).sqrt().floor();

            for row_index in first_visible_row..last_visible_row as usize {
                // each row length of the expanding triangle equal to twice the height plus 1
                let row_length = (row_index * 2) + 1;

                for index in 0..row_length {
                    if cell_arr[((row_index).pow(2) + index) as usize] {
                        // position is the 'camera' position + the row/column position in the cell array
                        let transform = c.transform.trans(
                            global_transform.x + (row_index as f64 - index as f64) * square_size,
                            global_transform.y + (square_size * row_index as f64),
                        );

                        // draw a black square if value at the cell arr index is true
                        rectangle(BLACK, square, transform, gl);
                    }
                }
            }
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.delta_ts.rotate_right(1);
        self.delta_ts[0] = 1.0 / args.dt
    }
}
