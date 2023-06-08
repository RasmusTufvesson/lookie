use piston_window::*;
use opengl_graphics::{OpenGL};
use pid::Pid;
use rand::{thread_rng, rngs::ThreadRng, Rng};
use glam::{
    DVec2,
};

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const ROTATION_AMOUNT: f64 = 0.004;
const MIN_ROT: f64 = -0.07;
const MAX_ROT: f64 = 0.07;
const DRIFT: f64 = 10.0;
const IMAGE_SCALE: f64 = 10.0;

fn gen_rand_pos(rng: &mut ThreadRng) -> DVec2 {
    DVec2::new(rng.gen::<f64>() * DRIFT, rng.gen::<f64>() * DRIFT)
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

    let mut texture_context = TextureContext {
        factory: window.factory.clone(),
        encoder: window.factory.create_command_buffer().into()
    };
    let texture: G2dTexture = Texture::from_path(
        &mut texture_context,
        "assets/image.png",
        Flip::None,
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

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(_args) = e.render_args() {
            window.draw_2d(&e, |c: Context, g, _| {
                clear(BLACK, g);
                image(&texture, c.transform.trans(mouse_pos.x + drift.x, mouse_pos.y + drift.y).rot_rad(rotation).scale(IMAGE_SCALE, IMAGE_SCALE), g);
            });
        }
        
        if let Some(args) = e.mouse_cursor_args() {
            let pos: DVec2 = args.into();
            let diff = pos - mouse_pos;
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
        }
    }
}
