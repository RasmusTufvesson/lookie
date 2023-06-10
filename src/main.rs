extern crate image as im;

use im::Rgba;
use piston_window::*;
use opengl_graphics::{OpenGL};
use pid::Pid;
use rand::{thread_rng, rngs::ThreadRng, Rng};
use glam::{
    DVec2,
    UVec2,
};

const BLACK_GL: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: Rgba<u8> = Rgba { 0: [255; 4] };
const BLACK: Rgba<u8> = Rgba { 0: [0, 0, 0, 255] };
const BLANK: Rgba<u8> = Rgba { 0: [0; 4] };
const ROTATION_AMOUNT: f64 = 0.004;
const MIN_ROT: f64 = -0.07;
const MAX_ROT: f64 = 0.07;
const DRIFT: f64 = 10.0;
const IMAGE_SCALE: f64 = 6.0;
const COLORS: [Rgba<u8>; 4] = [Rgba { 0: [255; 4] }, Rgba { 0: [184, 55, 55, 255] }, Rgba { 0: [75, 173, 64, 255] }, Rgba { 0: [53, 41, 186, 255] }];
const PREVIEW_TIME: f64 = 1.0;

fn gen_rand_pos(rng: &mut ThreadRng) -> DVec2 {
    DVec2::new(rng.gen::<f64>() * DRIFT - DRIFT / 2.0, rng.gen::<f64>() * DRIFT - DRIFT / 2.0)
}

fn draw_line(image: &mut im::ImageBuffer<im::Rgba<u8>, Vec<u8>>, start: DVec2, end: DVec2, radius: f64, color: Rgba<u8>) {
    let dx = (end.x - start.x).abs();
    let dy = (end.y - start.y).abs();
    let sx: f64 = if start.x < end.x { 1.0 } else { -1.0 };
    let sy: f64 = if start.y < end.y { 1.0 } else { -1.0 };
    let mut err = dx - dy;

    let mut x = start.x;
    let mut y = start.y;

    while x as u32 != end.x as u32 || y as u32 != end.y as u32 {

        let e2 = 2.0 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }

        for add_x in -radius as i32..=radius as i32 {
            let current_x = x + add_x as f64;
            for add_y in -radius as i32..=radius as i32 {
                let current_y = y + add_y as f64;

                if current_x >= 0.0 && current_x < image.width() as f64 && current_y >= 0.0 && current_y < image.height() as f64 {
                    image.put_pixel(current_x as u32, current_y as u32, color);
                }
            }
        }
    }
}

fn clear_image(image: &mut im::ImageBuffer<Rgba<u8>, Vec<u8>>, color: Rgba<u8>) {
    for pixel in image.pixels_mut() {
        *pixel = color;
    }
}

fn flood_fill(image: &mut im::ImageBuffer<Rgba<u8>, Vec<u8>>, point: DVec2, fill_color: Rgba<u8>) {
    let target_color = image.get_pixel(point.x as u32, point.y as u32).clone();

    if target_color == fill_color {
        return;
    }

    let mut stack: Vec<UVec2> = vec![];
    stack.push(UVec2::new(point.x as u32, point.y as u32));

    while let Some(point) = stack.pop() {
        if image.get_pixel(point.x, point.y) == &target_color {
            image.put_pixel(point.x, point.y, fill_color);

            if point.x > 0 {
                stack.push(point - UVec2::X);
            }
            if point.x < image.width() - 1 {
                stack.push(point + UVec2::X);
            }
            if point.y > 0 {
                stack.push(point - UVec2::Y);
            }
            if point.y < image.height() - 1 {
                stack.push(point + UVec2::Y);
            }
        }
    }
}

fn render_preview(image: &mut im::ImageBuffer<Rgba<u8>, Vec<u8>>, color: Rgba<u8>, radius: f64) {
    clear_image(image, BLACK);
    let middle = UVec2::from([image.width(), image.height()]) / 2;
    for add_x in -radius as i32..=radius as i32 {
        let current_x = middle.x as i32 + add_x;
        for add_y in -radius as i32..=radius as i32 {
            let current_y = middle.y as i32 + add_y;

            if current_x >= 0 && current_x < image.width() as i32 && current_y >= 0 && current_y < image.height() as i32 {
                image.put_pixel(current_x as u32, current_y as u32, color);
            }
        }
    }
}

struct Inputs {
    pub mouse_down: bool,
    pub right_mouse_down: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Normal,
    Fill,
}

fn main() {
    let opengl = OpenGL::V3_2;
    let (width, height) = (700, 500);
    let mut window: PistonWindow =
        WindowSettings::new("Lookie", (width, height))
        .resizable(false)
        .graphics_api(opengl)
        .build()
        .unwrap();
    window.window.window.set_cursor_visible(false);

    let mut canvas: im::ImageBuffer<im::Rgba<u8>, Vec<_>> = im::ImageBuffer::new(width, height);
    let mut texture_context = TextureContext {
        factory: window.factory.clone(),
        encoder: window.factory.create_command_buffer().into()
    };
    let draw_icon: G2dTexture = Texture::from_path(
        &mut texture_context,
        "assets/draw.png",
        Flip::None,
        &TextureSettings::new().filter(Filter::Nearest),
    ).unwrap();
    let erase_icon: G2dTexture = Texture::from_path(
        &mut texture_context,
        "assets/erase.png",
        Flip::None,
        &TextureSettings::new().filter(Filter::Nearest),
    ).unwrap();
    let fill_icon: G2dTexture = Texture::from_path(
        &mut texture_context,
        "assets/fill.png",
        Flip::None,
        &TextureSettings::new().filter(Filter::Nearest),
    ).unwrap();
    let mut canvas_texture: G2dTexture = Texture::from_image(
        &mut texture_context,
        &canvas,
        &TextureSettings::new().filter(Filter::Nearest),
    ).unwrap();
    let mut mouse_pos = DVec2::new(0.0, 0.0);
    let mut rotation = 0.0;
    let mut pid = Pid::new(0.0, 10.0);
    pid.p(3.0, 10.0);
    pid.i(0.2, 10.0);
    pid.d(0.2, 10.0);
    let mut rng = thread_rng();
    let mut target_pos = gen_rand_pos(&mut rng);
    let mut start_pos = gen_rand_pos(&mut rng);
    let mut drift_progress = 0.0;
    let mut drift = start_pos.clone();
    let mut inputs = Inputs { mouse_down: false, right_mouse_down: false };
    let mut line_radius: f64 = 0.0;
    let mut color = WHITE;
    let mut color_index: i32 = 0;
    let mut mode = Mode::Normal;
    let mut preview_timer = 0.0;
    let mut preview_image: im::ImageBuffer<im::Rgba<u8>, Vec<_>> = im::ImageBuffer::new(50, 50);
    let mut preview_texture: G2dTexture = Texture::from_image(
        &mut texture_context,
        &preview_image,
        &TextureSettings::new().filter(Filter::Nearest),
    ).unwrap();

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(_args) = e.render_args() {
            window.draw_2d(&e, |c: Context, g, device| {
                clear(BLACK_GL, g);
                canvas_texture.update(&mut texture_context, &canvas).unwrap();
                texture_context.encoder.flush(device);
                image(&canvas_texture, c.transform, g);
                if inputs.right_mouse_down {
                    image(&erase_icon, c.transform.trans(mouse_pos.x + drift.x, mouse_pos.y + drift.y).rot_rad(rotation).scale(IMAGE_SCALE, IMAGE_SCALE), g);
                } else {
                    match mode {
                        Mode::Normal => {
                            image(&draw_icon, c.transform.trans(mouse_pos.x + drift.x, mouse_pos.y + drift.y).rot_rad(rotation).scale(IMAGE_SCALE, IMAGE_SCALE), g);
                        }
                        Mode::Fill => {
                            image(&fill_icon, c.transform.trans(mouse_pos.x + drift.x, mouse_pos.y + drift.y).rot_rad(rotation).scale(IMAGE_SCALE, IMAGE_SCALE), g);
                        }
                    }
                }
                if preview_timer > 0.0 {
                    preview_texture.update(&mut texture_context, &preview_image).unwrap();
                    texture_context.encoder.flush(device);
                    image(&preview_texture, c.transform.trans((width-55) as f64, 5.0), g);
                }
            });
        }
        
        if let Some(args) = e.mouse_cursor_args() {
            let pos: DVec2 = DVec2::new((args[0]).clamp(0.0, width as f64 - 1.0), (args[1]).clamp(0.0, height as f64 - 1.0));
            let diff = pos - mouse_pos;
            if mode == Mode::Normal {
                if inputs.mouse_down {
                    draw_line(&mut canvas, mouse_pos, pos.clone(), line_radius, color);
                } else if inputs.right_mouse_down {
                    draw_line(&mut canvas, mouse_pos, pos.clone(), line_radius, BLANK);
                }
            }
            mouse_pos = pos;
            rotation += ((diff[0] - diff[1]) * ROTATION_AMOUNT).clamp(MIN_ROT, MAX_ROT);
        }
        
        if let Some(args) = e.update_args() {
            rotation += pid.next_control_output(rotation).output * args.dt;
            drift_progress += args.dt;
            if drift_progress > 1.0 {
                drift_progress = 0.0;
                start_pos = target_pos;
                target_pos = gen_rand_pos(&mut rng);
            }
            drift = start_pos.lerp(target_pos, drift_progress);
            if preview_timer > 0.0 {
                preview_timer -= args.dt;
            }
        }
        
        if let Some(args) = e.press_args() {
            if let Button::Mouse(button) = args {
                match button {
                    MouseButton::Left => {
                        inputs.mouse_down = true;
                        if mode == Mode::Fill {
                            if inputs.right_mouse_down {
                                flood_fill(&mut canvas, mouse_pos, BLANK);
                            } else {
                                flood_fill(&mut canvas, mouse_pos, color);
                            }
                        }
                    }
                    MouseButton::Right => {
                        inputs.right_mouse_down = true;
                    }
                    _ => {}
                }
            } else if let Button::Keyboard(key) = args {
                match key {
                    Key::Return => {
                        clear_image(&mut canvas, BLANK);
                    }
                    Key::Up => {
                        line_radius = (line_radius + 1.0).min(10.0);
                        preview_timer = PREVIEW_TIME;
                        render_preview(&mut preview_image, color, line_radius);
                    }
                    Key::Down => {
                        line_radius = (line_radius - 1.0).max(0.0);
                        preview_timer = PREVIEW_TIME;
                        render_preview(&mut preview_image, color, line_radius);
                    }
                    Key::Left => {
                        color_index -= 1;
                        if color_index < 0 {
                            color_index = COLORS.len() as i32 - 1;
                        }
                        color = COLORS[color_index as usize];
                        preview_timer = PREVIEW_TIME;
                        render_preview(&mut preview_image, color, line_radius);
                    }
                    Key::Right => {
                        color_index += 1;
                        if color_index >= COLORS.len() as i32 {
                            color_index = 0;
                        }
                        color = COLORS[color_index as usize];
                        preview_timer = PREVIEW_TIME;
                        render_preview(&mut preview_image, color, line_radius);
                    }
                    Key::B => {
                        mode = Mode::Fill;
                    }
                    Key::N => {
                        mode = Mode::Normal;
                    }
                    _ => {}
                }
            }
        }
        
        if let Some(args) = e.release_args() {
            if let Button::Mouse(button) = args {
                match button {
                    MouseButton::Left => {
                        inputs.mouse_down = false;
                    }
                    MouseButton::Right => {
                        inputs.right_mouse_down = false;
                    }
                    _ => {}
                }
            }
        }
    }
}
