use log::error;
use smithay_client_toolkit::{
    reexports::client::{
        protocol::{wl_output::WlOutput, wl_shm::Format},
        DispatchError, EventQueue,
    },
    shm::{
        slot::{Buffer, CreateBufferError},
        CreatePoolError,
    },
    wlr_screencopy::{BufferType, FrameStatus},
};

use crate::state::State;

#[derive(Debug, thiserror::Error)]
pub enum FrameCaptureError {
    #[error("Queue dispatch error when capturing frame: {0}")]
    QueueDispatch(#[from] DispatchError),
    #[error("Captured frame don't have a compatible buffer type")]
    NoCompatibleBuffer,
    #[error("Unable to create wl_buffer: {0}")]
    CreateBuffer(#[from] CreateBufferError),
    #[error("Error when capturing frame")]
    CaptureError,
    #[error("Error creating SlotPool: {0}")]
    CreatePool(#[from] CreatePoolError),
}

/// A frame captured and copied to a wl_buffer
pub struct CapturedFrame {
    buf: Buffer,
    format: Format,
    width: u32,
    height: u32,
}

impl CapturedFrame {
    /// get a mutable reference to the underlying buffer data
    pub fn data_mut<'pool>(&mut self, state: &'pool mut State) -> Option<&'pool mut [u8]> {
        self.buf.slot().canvas(&mut state.slot_pool)
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Capture a frame from given output via wlr_screencopy
pub fn capture_output(
    state: &mut State,
    queue: &mut EventQueue<State>,
    output: &WlOutput,
) -> Result<CapturedFrame, FrameCaptureError> {
    let qh = queue.handle();

    let frame = state.wlr_screencopy_state.capture_output(output, &qh);

    // get buffer infos for the frame
    queue.roundtrip(state)?;
    let buffer_type = frame
        .buffer_types()
        .iter()
        .find(|t| {
            if let BufferType::WlShm { .. } = t {
                true
            } else {
                false
            }
        })
        .cloned()
        .ok_or(FrameCaptureError::NoCompatibleBuffer)?;

    let cap_frame = match buffer_type {
        BufferType::WlShm {
            format,
            width,
            height,
            stride,
        } => {
            // Put captured frame into a buffer
            let (buf, _) = state.slot_pool.create_buffer(
                width as i32,
                height as i32,
                stride as i32,
                format,
            )?;
            frame.copy(buf.wl_buffer());
            CapturedFrame {
                buf,
                format,
                width,
                height,
            }
        }
        BufferType::LinuxDmabuf { .. } => todo!("Unsupported for now"),
    };

    // loop until we get all events
    loop {
        match frame.status() {
            FrameStatus::NotReady => queue.blocking_dispatch(state)?,
            FrameStatus::Failed => return Err(FrameCaptureError::CaptureError),
            FrameStatus::Ready((_tv_sec_hi, _tv_sec_lo, _tv_nsec)) => break,
        };
    }

    Ok(cap_frame)
}
