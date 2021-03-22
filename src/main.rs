extern crate sdl2;

// TODO :
// Investigate why FPS counter always seem to be 1-2 frames too low??
// Cache the glyphs from ttf for more fine grained control over color and formatting per glyph

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::{Font, FontStyle};
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;
use std::path::Path;
use std::{thread, time};
use time::{Duration, Instant};

#[macro_use]
extern crate clap;
use clap::App;

const STD_CLS: &[(&str, Color)] = &[("Blue", Color::RGB(100, 160, 230))];
const STD_TTF: &[(&str, &str)] = &[("Standard", "rsc/disposabledroid-bb.regular.ttf")]; // https://www.1001fonts.com/disposabledroid-bb-font.html

pub const PIXEL_SIZE: u32 = 4;
pub const WIDTH: u32 = 2560 / PIXEL_SIZE;
pub const HEIGHT: u32 = 1440 / PIXEL_SIZE;

pub struct RenderData<'a> {
    pub x: usize,
    pub y: usize,
    pub z: isize,
    pub w: usize,
    pub h: usize,
    pub borrowed_tex: Option<&'a sdl2::render::Texture<'a>>,
    pub owned_tex: Option<sdl2::render::Texture<'a>>,
}

#[derive(Clone, PartialEq)]
pub struct TextBlock {
    pub text: String,
    pub x: usize,
    pub y: usize,
    pub z: isize,
    pub w: usize,
    pub h: usize,
    pub color: Color,
    pub fontname: String,
    pub fontsize: f64,
}

#[derive(Copy, Clone, PartialEq)]
pub enum State {
    Paused,
    Playing,
}

pub struct World {
    pub playground: [bool; (WIDTH * HEIGHT) as usize],
    pub state: State,
    pub text: TextBlock,
}

impl World {
    pub fn new() -> World {
        World {
            playground: [false; (WIDTH * HEIGHT) as usize],
            state: State::Paused,
            text: TextBlock {
                text: format!("UPS: {:.2}\nFPS: {:.2}", 0.0, 0.0),
                x: 0,
                y: 0,
                z: 0,
                w: 100,
                h: 100,
                color: Color::RGB(255, 255, 255),
                fontname: String::from("Standard"),
                fontsize: 14.0,
            },
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<bool> {
        if x >= 0 && y >= 0 && (x as u32) < WIDTH && (y as u32) < HEIGHT {
            Some(self.playground[(x as u32 + (y as u32) * WIDTH) as usize])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut bool> {
        if x >= 0 && y >= 0 && (x as u32) < WIDTH && (y as u32) < HEIGHT {
            Some(&mut self.playground[(x as u32 + (y as u32) * WIDTH) as usize])
        } else {
            None
        }
    }

    pub fn toggle_state(&mut self) {
        self.state = match self.state {
            State::Paused => State::Playing,
            State::Playing => State::Paused,
        }
    }

    pub fn update(&mut self) {
        let mut new_playground = self.playground;
        for (u, square) in new_playground.iter_mut().enumerate() {
            let u = u as u32;
            let x = u % WIDTH;
            let y = u / WIDTH;
            let mut count: u32 = 0;
            for i in -1..2 {
                for j in -1..2 {
                    if !(i == 0 && j == 0) {
                        let peek_x: i32 = (x as i32) + i;
                        let peek_y: i32 = (y as i32) + j;
                        if let Some(true) = self.get(peek_x, peek_y) {
                            count += 1;
                        }
                    }
                }
            }
            if count > 3 || count < 2 {
                *square = false;
            } else if count == 3 {
                *square = true;
            } else if count == 2 {
                *square = *square;
            }
        }
        self.playground = new_playground;
    }

    pub fn clear(&mut self) {
        self.playground = [false; (WIDTH * HEIGHT) as usize];
    }
}

impl<'a> IntoIterator for &'a World {
    type Item = &'a bool;
    type IntoIter = ::std::slice::Iter<'a, bool>;
    fn into_iter(self) -> ::std::slice::Iter<'a, bool> {
        self.playground.iter()
    }
}

fn cache_pixel_texture<'a>(
    canvas: &mut Canvas<Window>,
    texture_creator: &'a TextureCreator<WindowContext>,
    color: Color,
) -> Result<Texture<'a>, String> {
    let mut target_texture = texture_creator
        .create_texture_target(None, PIXEL_SIZE, PIXEL_SIZE)
        .map_err(|e| e.to_string())?;
    canvas
        .with_texture_canvas(&mut target_texture, |texture_canvas| {
            texture_canvas.set_draw_color(color);
            for i in 0..PIXEL_SIZE {
                for j in 0..PIXEL_SIZE {
                    // drawing pixel by pixel isn't very effective, but we only do it once and store
                    // the texture afterwards so it's still alright!
                    // this doesn't mean anything, there was some trial and serror to find
                    // something that wasn't too ugly
                    texture_canvas
                        .draw_point(Point::new(i as i32, j as i32))
                        .expect("could not draw point");
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(target_texture)
}

pub fn prepare_pixels<'a>(
    world: &World,
    pxtx: &'a HashMap<&str, Texture<'a>>,
    rendq: &mut Vec<RenderData<'a>>,
) {
    for (i, unit) in (&world).into_iter().enumerate() {
        let i = i as u32;
        if *unit {
            let data = RenderData {
                x: ((i % WIDTH) * PIXEL_SIZE) as usize,
                y: ((i / WIDTH) * PIXEL_SIZE) as usize,
                w: PIXEL_SIZE as usize,
                h: PIXEL_SIZE as usize,
                z: 0,
                borrowed_tex: Some(&pxtx["Blue"]),
                owned_tex: None,
            };
            rendq.push(data);
        }
    }
}

pub fn prepare_text<'a>(
    tex_creator: &'a TextureCreator<WindowContext>,
    text: &TextBlock,
    ttf_atlas: &HashMap<String, Font>,
    rendq: &mut Vec<RenderData<'a>>,
) -> Result<(), String> {
    // render a surface, and convert it to a texture bound to the canvas
    let font = &ttf_atlas[text.fontname.as_str()];
    let surface = font
        .render(text.text.as_str())
        .solid(text.color)
        .map_err(|e| e.to_string())?;
    let texture = tex_creator
        .create_texture_from_surface(&surface)
        .map_err(|e| e.to_string())?;

    let font_size = font.size_of(text.text.as_str()).unwrap();

    let data = RenderData {
        x: text.x,
        y: text.y,
        w: font_size.0 as usize,
        h: font_size.1 as usize,
        z: text.z,
        borrowed_tex: None,
        owned_tex: Some(texture),
    };

    rendq.push(data);

    // acc += font_size.1 as usize;
    // }

    Ok(())
}

pub fn draw(canvas: &mut Canvas<sdl2::video::Window>, data: Vec<RenderData>) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(20, 0, 0));
    canvas.clear();

    for rend in data {
        if rend.borrowed_tex.is_some() {
            canvas.copy(
                rend.borrowed_tex.unwrap(),
                None,
                Rect::new(rend.x as i32, rend.y as i32, rend.w as u32, rend.h as u32),
            )?;
        } else {
            canvas.copy(
                &rend.owned_tex.unwrap(),
                None,
                Rect::new(rend.x as i32, rend.y as i32, rend.w as u32, rend.h as u32),
            )?;
        };
    }

    canvas.present();

    Ok(())
}

pub fn main() -> Result<(), String> {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let max_fps = if matches.is_present("max-fps") {
        matches
            .value_of("max-fps")
            .unwrap()
            .parse()
            .unwrap_or(std::f64::NEG_INFINITY)
    } else {
        std::f64::INFINITY
    };

    if max_fps == std::f64::NEG_INFINITY {
        return Err("max-fps is not convertible to a float value".to_owned());
    }

    let frame_period = 1.0f64 / max_fps;

    println!(
        "Max FPS set to {} (frame period is {})",
        max_fps, frame_period
    );

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Tetroid", WIDTH * PIXEL_SIZE, HEIGHT * PIXEL_SIZE)
        .position_centered()
        // .opengl() // ??
        .fullscreen_desktop()
        // .fullscreen()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .target_texture()
        // .present_vsync()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    println!("Using SDL_Renderer \"{}\"", canvas.info().name);

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    // Load the fonts
    let mut ttf_atlas: HashMap<String, Font> = HashMap::new();
    for ttf in STD_TTF {
        ttf_atlas.insert(
            String::from(ttf.0),
            ttf_context.load_font(Path::new(ttf.1), 44)?,
        );
        ttf_atlas
            .get_mut(&String::from(ttf.0))
            .unwrap()
            .set_style(FontStyle::NORMAL);
    }

    let texture_creator: TextureCreator<_> = canvas.texture_creator();

    // cache some standard colors
    let mut pxtx: HashMap<&str, Texture> = HashMap::new();
    for col in STD_CLS {
        pxtx.insert(
            col.0,
            cache_pixel_texture(&mut canvas, &texture_creator, col.1)?,
        );
    }
    let mut world = World::new();

    let mut rendq: Vec<RenderData> = Vec::new();
    prepare_pixels(&world, &pxtx, &mut rendq);
    prepare_text(&texture_creator, &world.text, &ttf_atlas, &mut rendq)?;
    draw(&mut canvas, rendq)?; // Initial draw of the canvas

    let mut event_pump = sdl_context.event_pump()?;

    let mut mousebtn_down = false;
    let mut movev: Vec<(i32, i32)> = vec![];
    let mut last_m: Option<(i32, i32)> = None;
    let mut frame_no: u64 = 0;
    let mut tic = Instant::now();
    let mut gfx_tic = Instant::now();
    let mut phy_tic = Instant::now();
    let mut ups = 0.0f64;
    // let mut current_mouse: (i32, i32) = (0, 0);

    'running: loop {
        let delta = tic.elapsed().as_secs_f64();
        if delta > frame_period {
            tic += Duration::from_secs_f64(delta);

            let mut rendq: Vec<RenderData> = Vec::new();
            prepare_pixels(&world, &pxtx, &mut rendq);
            prepare_text(&texture_creator, &world.text, &ttf_atlas, &mut rendq)?;
            draw(&mut canvas, rendq)?; // Initial draw of the canvas

            // update FPS counter
            let gfx_delta = gfx_tic.elapsed().as_secs_f64();
            if gfx_delta > 1.0 / 2.0 {
                gfx_tic += Duration::from_secs_f64(gfx_delta);
                let fps = (frame_no as f64) / gfx_delta;
                world.text.text = format!("UPS: {:.2}\nFPS: {:.2}", ups, fps);
                // print!("\x1B[2J"); // clear terminal
                // println!("ups: {}", ups);
                // println!("fps: {}", fps);
                // println!(
                //     "mouse: {:?}",
                //     (
                //         current_mouse.0 / PIXEL_SIZE as i32,
                //         current_mouse.1 / PIXEL_SIZE as i32
                //     )
                // );
                frame_no = 0;
            } else {
                frame_no += 1;
            }
        }

        let phy_delta = phy_tic.elapsed().as_secs_f64();
        if phy_delta > 1.0 / 60.0 {
            phy_tic += Duration::from_secs_f64(phy_delta);
            ups = 1.0 / phy_delta;

            // get the inputs here
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Q),
                        ..
                    } => break 'running,

                    Event::KeyDown {
                        keycode: Some(Keycode::Space),
                        repeat: false,
                        ..
                    } => {
                        world.toggle_state();
                    }

                    Event::KeyDown {
                        keycode: Some(Keycode::E),
                        repeat: false,
                        ..
                    } => {
                        world.clear();
                    }

                    Event::MouseButtonDown {
                        x,
                        y,
                        mouse_btn: MouseButton::Left,
                        ..
                    } => {
                        mousebtn_down = true;
                        movev.push((x, y));
                        last_m = Some((x, y));
                    }

                    Event::MouseMotion { x, y, .. } => {
                        if mousebtn_down {
                            movev.push((x, y));
                        }
                        // current_mouse = (x, y);
                    }

                    Event::MouseButtonUp {
                        mouse_btn: MouseButton::Left,
                        ..
                    } => {
                        mousebtn_down = false;
                        last_m = None;
                        movev.clear();
                    }

                    _ => {}
                }
            }

            if mousebtn_down {
                for m in movev.drain(0..movev.len()) {
                    if last_m.is_some() {
                        let last_m = last_m.unwrap();

                        // Bresenham's line algorithm
                        let mut x0 = last_m.0;
                        let x1 = m.0;
                        let mut y0 = last_m.1;
                        let y1 = m.1;

                        let dx = (x1 - x0).abs();
                        let dy = -(y1 - y0).abs();
                        let sx = if x0 < x1 { 1 } else { -1 };
                        let sy = if y0 < y1 { 1 } else { -1 };
                        let mut err = dx + dy;
                        loop {
                            match world.get_mut(
                                (x0 as u32 / PIXEL_SIZE) as i32,
                                (y0 as u32 / PIXEL_SIZE) as i32,
                            ) {
                                Some(square) => {
                                    if *square == false {
                                        *square = !(*square);
                                    }
                                }
                                None => {}
                            };

                            if (x0 == x1) && (y0 == y1) {
                                break;
                            };

                            let e2 = 2 * err;
                            if e2 >= dy {
                                err += dy;
                                x0 += sx;
                            }
                            if e2 <= dx {
                                err += dx;
                                y0 += sy;
                            }
                        }
                    }

                    last_m = Some(m);
                }
            }

            if world.state == State::Playing {
                world.update();
            }
        }
        thread::yield_now(); // cpu friendly
    }

    Ok(())
}
