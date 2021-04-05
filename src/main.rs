mod engine;
use engine::*;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::ttf::{Font, FontStyle};

use sdl2::render::{Texture, TextureCreator};

#[macro_use]
extern crate clap;

use std::collections::HashMap;
use std::path::Path;
use std::{thread, time};
use time::{Duration, Instant};

pub fn main() -> Result<(), String> {
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();
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
        // .vulkan() // ??
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
    prepare_text(&texture_creator, &world.m_text, &ttf_atlas, &mut rendq)?;
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
            prepare_text(&texture_creator, &world.m_text, &ttf_atlas, &mut rendq)?;
            draw(&mut canvas, rendq)?; // Initial draw of the canvas

            // update FPS counter
            let gfx_delta = gfx_tic.elapsed().as_secs_f64();
            if gfx_delta > 1.0 / 2.0 {
                gfx_tic += Duration::from_secs_f64(gfx_delta);
                let fps = (frame_no as f64) / gfx_delta;
                world.m_text.text = format!(
                    "UPS: {:.2}\nFPS: {:.2}\nnum: {}",
                    ups,
                    fps,
                    world.num_alive()
                );
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

            if world.m_state == State::Playing {
                world.update();
            }
        }
        thread::yield_now(); // cpu friendly
    }

    Ok(())
}
