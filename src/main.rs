use image::{ImageBuffer, RgbaImage};
use smithay_client_toolkit::{
    delegate_output, delegate_registry, delegate_shm, delegate_wlr_screencopy,
    output::{OutputHandler, OutputState},
    reexports::client::{
        globals::registry_queue_init, protocol::wl_output::WlOutput, Connection, QueueHandle,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shm::{slot::SlotPool, Shm, ShmHandler},
    wlr_screencopy::{BufferType, FrameStatus, WlrScreencopyHandler, WlrScreencopyState},
};
use tracing::error;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let conn = Connection::connect_to_env().expect("Unable to connect to the wayland socket.");

    let (globals, mut queue) = registry_queue_init(&conn).expect("Unable to start registry");
    let qh: QueueHandle<State> = queue.handle();

    let mut state = State {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        wlr_screencopy_state: WlrScreencopyState::new(&globals, &qh),
        shm_state: Shm::bind(&globals, &qh).unwrap(),
    };

    // Get outputs data
    queue.roundtrip(&mut state).unwrap();
    state
        .output_state
        .outputs()
        .filter_map(|o| state.output_state.info(&o))
        .for_each(|info| println!("{:?}", info));

    let frame = state
        .wlr_screencopy_state
        .capture_output(&state.output_state.outputs().next().unwrap(), &qh);

    // get buffer infos for the frame
    queue.roundtrip(&mut state).unwrap();
    let BufferType::WlShm {
        format,
        width,
        height,
        stride,
    } = frame
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
        .unwrap()
    else {
        return;
    };

    let mut pool = SlotPool::new(1, &state.shm_state).unwrap();
    let (wl_buffer, buf) = pool
        .create_buffer(width as i32, height as i32, stride as i32, format)
        .unwrap();

    frame.copy(wl_buffer.wl_buffer());

    // Blocking until we get events
    queue.blocking_dispatch(&mut state).unwrap();

    match frame.status() {
        FrameStatus::NotReady => return error!("Frame is not ready"),
        FrameStatus::Failed => return error!("Failed to get frame"),
        _ => (),
    }

    println!("{:?}", format);

    let img: RgbaImage = ImageBuffer::from_raw(width, height, buf.to_vec()).unwrap();
    img.save("out.png").unwrap();
}

struct State {
    registry_state: RegistryState,
    output_state: OutputState,
    wlr_screencopy_state: WlrScreencopyState,
    shm_state: Shm,
}

// wl_registry

delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers! {
        OutputState,
    }
}

// wl_output

delegate_output!(State);

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
    }
}

// wl_shm

delegate_shm!(State);

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

// zwlr_screencopy_manager_v1

delegate_wlr_screencopy!(State);

impl WlrScreencopyHandler for State {
    fn wlr_screencopy_state(&mut self) -> &mut WlrScreencopyState {
        &mut self.wlr_screencopy_state
    }
}
