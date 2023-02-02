//! This is a simple packetized ray tracer example which demonstrates
//! interopability with structs in Rust and ISPC.

#[macro_use]
extern crate ispc;
extern crate image;
extern crate rand;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate docopt;

use std::time::Instant;

use docopt::Docopt;
use rand::distributions::Standard;
use rand::{thread_rng, Rng};

use crate::camera::Camera;
use crate::scene::Scene;

mod camera;
mod geom;
mod lights;
mod material;
mod scene;
mod vec3f;

ispc_module!(rt);

const USAGE: &str = "
Usage:
  rt <scene> [options]
  rt (-h | --help)

Options:
  -o OUT        Specify a file to writing the render to, defaults to 'rt.png'.
  -h, --help    Show this message.
";

#[derive(Deserialize)]
pub struct Args {
    arg_scene: String,
    flag_o: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let scene = Scene::load(&args.arg_scene[..]);
    let mut framebuffer = vec![0.0; scene.width * scene.height * 3];
    let mut srgb_img_buf = vec![0u8; scene.width * scene.height * 3];
    // We need a random seed for each scanline of the image
    let scanline_seeds: Vec<i32> = thread_rng()
        .sample_iter(&Standard)
        .take(scene.height)
        .collect();
    unsafe {
        let geom: Vec<_> = scene.geometry.iter().map(|x| x.ispc_equiv()).collect();
        let start = Instant::now();
        rt::render(
            &scene.camera as *const Camera,
            geom.as_ptr(),
            geom.len() as i32,
            scene.light.ispc_equiv(),
            scanline_seeds.as_ptr(),
            scene.width as i32,
            scene.height as i32,
            framebuffer.as_mut_ptr(),
            scene.n_samples as i32,
        );
        let elapsed = start.elapsed();
        println!(
            "Rendering took {}s",
            elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
        );
        rt::framebuffer_to_srgb(
            framebuffer.as_ptr(),
            srgb_img_buf.as_mut_ptr(),
            scene.width as i32,
            scene.height as i32,
        );
    }
    let out_file = match args.flag_o {
        Some(s) => s,
        None => String::from("rt.png"),
    };
    match image::save_buffer(
        &out_file[..],
        &srgb_img_buf[..],
        scene.width as u32,
        scene.height as u32,
        image::ColorType::Rgb8,
    ) {
        Ok(_) => println!("Rendered image saved to {out_file}"),
        Err(e) => panic!("Error saving image: {e}"),
    };
}
