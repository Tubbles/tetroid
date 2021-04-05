use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::{Canvas, Texture, TextureCreator};
// use sdl2::render::{Texture, TextureCreator};
use sdl2::ttf::{Font, FontStyle};
use sdl2::video::{Window, WindowContext};

use std::collections::HashMap;
use std::path::Path;

const STD_CLS: &[(&str, Color)] = &[("Blue", Color::RGB(100, 160, 230))];
const STD_TTF: &[(&str, &str)] = &[("Standard", "rsc/disposabledroid-bb.regular.ttf")]; // https://www.1001fonts.com/disposabledroid-bb-font.html

pub fn init_res_contexts<'a>(
    canvas: &mut Canvas<Window>,
    pixel_size: u32,
) -> Result<
    (
        HashMap<String, Font>,
        TextureCreator<WindowContext>,
        HashMap<String, Texture>,
    ),
    String,
> {
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    // Load the fonts
    let mut ttf_atlas = HashMap::new();
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

    let texture_creator = canvas.texture_creator();

    // cache some standard colors
    let mut pixel_altas = HashMap::new();
    for col in STD_CLS {
        pixel_altas.insert(
            String::from(col.0),
            cache_pixel_texture(canvas, &texture_creator, col.1, pixel_size)?,
        );
    }

    Ok((ttf_atlas, texture_creator, pixel_altas))
}

fn cache_pixel_texture<'a>(
    canvas: &'a mut Canvas<Window>,
    texture_creator: &'a TextureCreator<WindowContext>,
    color: Color,
    pixel_size: u32,
) -> Result<Texture<'a>, String> {
    let mut target_texture = texture_creator
        .create_texture_target(None, pixel_size, pixel_size)
        .map_err(|e| e.to_string())?;
    canvas
        .with_texture_canvas(&mut target_texture, |texture_canvas| {
            texture_canvas.set_draw_color(color);
            for i in 0..pixel_size {
                for j in 0..pixel_size {
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
