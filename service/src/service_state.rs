use common::{Mode, config::MiriConfig};
use niri_ipc::state::EventStreamState;
use std::collections::HashMap;

use crate::niri_ipc_utils::get_focused_workspace;

pub struct ServiceState {
    pub workspace_modes: WorkspaceModes,
    pub config: MiriConfig,
}

impl ServiceState {
    pub fn new(config: MiriConfig) -> Self {
        ServiceState {
            workspace_modes: WorkspaceModes::new(config.default_workspace_mode),
            config,
        }
    }
}

pub struct WorkspaceModes {
    // output name and index used as key
    // FIXME: solve case of output name being the same
    modes: HashMap<(String, u8), Mode>,
    default_mode: Mode,
}

impl WorkspaceModes {
    pub fn new(default_mode: Mode) -> Self {
        WorkspaceModes {
            modes: HashMap::new(),
            default_mode,
        }
    }

    pub fn get_mode(&self, output: &str, index: u8) -> Mode {
        let Some(current_mode) = self.modes.get(&(output.to_string(), index)) else {
            return self.default_mode;
        };
        *current_mode
    }

    pub fn set_mode(&mut self, output: &str, index: u8, mode: Mode) {
        self.modes.insert((output.to_string(), index), mode);
    }

    pub fn set_mode_on_focused_workspace(&mut self, event_state: &EventStreamState, mode: Mode) {
        let Some(focused_workspace) = get_focused_workspace(event_state) else {
            eprintln!("Failed to get focused workspace");
            return;
        };

        let Some(output) = focused_workspace.output.as_ref() else {
            eprintln!("Focused workspace has no output");
            return;
        };

        self.set_mode(output, focused_workspace.idx, mode);
    }

    pub fn cycle_mode(&mut self, output: &str, index: u8) {
        let current_mode = self.get_mode(output, index);

        let new_mode = current_mode.cycle();
        self.set_mode(output, index, new_mode);
    }

    pub fn cycle_mode_on_focused_workspace(&mut self, event_state: &EventStreamState) {
        let Some(focused_workspace) = get_focused_workspace(event_state) else {
            eprintln!("Failed to get focused workspace");
            return;
        };

        let Some(output) = focused_workspace.output.as_ref() else {
            eprintln!("Focused workspace has no output");
            return;
        };

        self.cycle_mode(output, focused_workspace.idx);
    }
}
