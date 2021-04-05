extern crate sdl2;

// TODO :
// Investigate why FPS counter always seem to be 1-2 frames too low??
// Cache the glyphs from ttf for more fine grained control over color and formatting per glyph
// If rendering becomes too slow/ineffective we might need to interface some gfx lib directly

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::Font;
use sdl2::video::WindowContext;

use std::collections::HashMap;

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
    pub width: u32,
    pub height: u32,
    pub playground: Vec<bool>,
    pub state: State,
    pub text: TextBlock,
}

impl Default for World {
    fn default() -> Self {
        World::new(60, 40)
    }
}

impl World {
    pub fn new(width: u32, height: u32) -> World {
        World {
            width,
            height,
            playground: vec![false; (width * height) as usize],
            state: State::Paused,
            text: TextBlock {
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
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            Some(self.playground[(x as u32 + (y as u32) * self.width) as usize])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut bool> {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            Some(&mut self.playground[(x as u32 + (y as u32) * self.width) as usize])
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
        let mut new_playground = self.playground.clone();
        for (u, square) in new_playground.iter_mut().enumerate() {
            let u = u as u32;
            let x = u % self.width;
            let y = u / self.width;
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
        self.playground = new_playground;
    }

    pub fn clear(&mut self) {
        self.playground = vec![false; (self.width * self.height) as usize];
    }

    pub fn num_alive(&mut self) -> usize {
        self.playground.iter().filter(|&x| *x).count()
    }
}

impl<'a> IntoIterator for &'a World {
    type Item = &'a bool;
    type IntoIter = ::std::slice::Iter<'a, bool>;
    fn into_iter(self) -> ::std::slice::Iter<'a, bool> {
        self.playground.iter()
    }
}

pub fn prepare_pixels<'a>(
    world: &World,
    pxtx: &'a HashMap<String, Texture<'a>>,
    rendq: &mut Vec<RenderData<'a>>,
    width: u32,
    pixel_size: u32,
) {
    for (i, unit) in (&world).into_iter().enumerate() {
        let i = i as u32;
        if *unit {
            let data = RenderData {
                x: ((i % width) * pixel_size) as usize,
                y: ((i / width) * pixel_size) as usize,
                w: pixel_size as usize,
                h: pixel_size as usize,
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
