use std::process::exit;

use image::{buffer::ConvertBuffer, DynamicImage, ImageBuffer, RgbImage, RgbaImage, XrgbImage};
use log::error;
use rgb::{Bgra, ComponentSlice, FromSlice, Rgba};
use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init, protocol::wl_shm::Format, Connection, QueueHandle,
};
use state::State;

mod state;
mod wlr_screencopy;

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().expect("Unable to connect to the wayland socket.");

    let (globals, mut queue) = registry_queue_init(&conn).expect("Unable to start registry");
    let qh: QueueHandle<State> = queue.handle();

    let mut state = match State::new(&globals, &qh) {
        Ok(s) => s,
        Err(e) => return error!("Error creating global state: {}", e),
    };

    // Get outputs data
    queue.roundtrip(&mut state).unwrap();

    // Capture first output
    let output = state.output_state.outputs().next().expect("No outputs");
    let mut frame = match wlr_screencopy::capture_output(&mut state, &mut queue, &output) {
        Ok(frame) => frame,
        Err(e) => {
            error!("Capturing output {:?} faile: {}", output, e);
            exit(1);
        }
    };

    let buf = frame
        .data_mut(&mut state)
        .expect("Unable to access buffer")
        .to_vec();
    let format = frame.format();
    let img: DynamicImage = match format {
        Format::Rgba8888 => {
            let img: RgbaImage =
                ImageBuffer::from_raw(frame.width(), frame.height(), buf.to_vec()).unwrap();
            img.into()
        }
        Format::Xrgb8888 => {
            // Convert into RGBA8888
            let buf: Vec<u8> = buf
                // .as_argb()
                .as_bgra()
                .iter()
                .cloned()
                .map(Into::<Rgba<u8>>::into)
                .map(|p| p.as_slice().to_vec())
                .flatten()
                .collect();
            let img: RgbaImage = ImageBuffer::from_raw(frame.width(), frame.height(), buf).unwrap();
            img.into()
        }
        _ => {
            error!("Frame format ({:?}) unsupported", format);
            exit(1);
        }
    };
    let output_info = state
        .output_state
        .info(&output)
        .expect("Unable to query output informations");

    img.save(format!("{}.png", output_info.name.unwrap()))
        .unwrap();
}
