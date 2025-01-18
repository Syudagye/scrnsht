use smithay_client_toolkit::{
    delegate_output, delegate_registry, delegate_shm, delegate_wlr_screencopy,
    output::{OutputHandler, OutputState},
    reexports::client::{
        globals::{BindError, GlobalList},
        protocol::wl_output::WlOutput,
        Connection, QueueHandle,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shm::{slot::SlotPool, CreatePoolError, Shm, ShmHandler},
    wlr_screencopy::{WlrScreencopyHandler, WlrScreencopyState},
};

#[derive(Debug, thiserror::Error)]
pub enum StateCreateError {
    #[error(transparent)]
    Bind(#[from] BindError),
    #[error("Error creating SlotPool: {0}")]
    CreatePool(#[from] CreatePoolError),
}

/// Global wayland state
pub struct State {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub wlr_screencopy_state: WlrScreencopyState,
    pub shm_state: Shm,

    pub slot_pool: SlotPool,
}

impl State {
    pub fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> Result<Self, StateCreateError> {
        let registry_state = RegistryState::new(globals);
        let output_state = OutputState::new(globals, qh);
        let wlr_screencopy_state = WlrScreencopyState::new(globals, qh)?;
        let shm_state = Shm::bind(globals, qh)?;
        let slot_pool = SlotPool::new(1, &shm_state)?;

        Ok(Self {
            registry_state,
            output_state,
            wlr_screencopy_state,
            shm_state,
            slot_pool,
        })
    }
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
