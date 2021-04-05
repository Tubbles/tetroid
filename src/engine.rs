extern crate sdl2;

// TODO :
// Investigate why FPS counter always seem to be 1-2 frames too low??
// Cache the glyphs from ttf for more fine grained control over color and formatting per glyph
// If rendering becomes too slow/ineffective we might need to interface some gfx lib directly

use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};

use std::collections::HashMap;

pub const STD_CLS: &[(&str, Color)] = &[("Blue", Color::RGB(100, 160, 230))];
pub const STD_TTF: &[(&str, &str)] = &[("Standard", "rsc/disposabledroid-bb.regular.ttf")]; // https://www.1001fonts.com/disposabledroid-bb-font.html

pub const PIXEL_SIZE: u32 = 2;
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
    pub m_playground: [bool; (WIDTH * HEIGHT) as usize],
    pub m_state: State,
    pub m_text: TextBlock,
}

impl Default for World {
    fn default() -> Self {
        World::new()
    }
}

impl World {
    pub fn new() -> World {
        World {
            m_playground: [false; (WIDTH * HEIGHT) as usize],
            m_state: State::Paused,
            m_text: TextBlock {
                text: format!("UPS: {:.2}\nFPS: {:.2}\nnum: {}", 0.0, 0.0, 0),
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
            Some(self.m_playground[(x as u32 + (y as u32) * WIDTH) as usize])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut bool> {
        if x >= 0 && y >= 0 && (x as u32) < WIDTH && (y as u32) < HEIGHT {
            Some(&mut self.m_playground[(x as u32 + (y as u32) * WIDTH) as usize])
        } else {
            None
        }
    }

    pub fn toggle_state(&mut self) {
        self.m_state = match self.m_state {
            State::Paused => State::Playing,
            State::Playing => State::Paused,
        }
    }

    pub fn update(&mut self) {
        let mut new_playground = self.m_playground;
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
            if !(2..4).contains(&count) {
                *square = false;
            } else if count == 3 {
                *square = true;
            }
        }
        self.m_playground = new_playground;
    }

    pub fn clear(&mut self) {
        self.m_playground = [false; (WIDTH * HEIGHT) as usize];
    }

    pub fn num_alive(&mut self) -> usize {
        self.m_playground.iter().filter(|&x| *x).count()
    }
}

impl<'a> IntoIterator for &'a World {
    type Item = &'a bool;
    type IntoIter = ::std::slice::Iter<'a, bool>;
    fn into_iter(self) -> ::std::slice::Iter<'a, bool> {
        self.m_playground.iter()
    }
}

pub fn cache_pixel_texture<'a>(
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
