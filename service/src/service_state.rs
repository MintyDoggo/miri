use common::{Mode, config::MiriConfig};
use niri_ipc::state::EventStreamState;
use std::collections::HashMap;

pub struct ServiceState {
    pub previous_layout: Layout,
    pub current_layout: Layout,
    pub config: MiriConfig,
}

impl ServiceState {
    pub fn new(config: MiriConfig) -> Self {
        ServiceState {
            previous_layout: Layout::new(config.default_workspace_mode),
            current_layout: Layout::new(config.default_workspace_mode),
            config,
        }
    }
}
#[derive(Debug)]
pub struct Layout {
    // output name and index used as key
    // FIXME: solve case of output name being the same
    pub workspaces: HashMap<(String, u8), MiriWorkspace>,
    pub default_mode: Mode,
}

impl Layout {
    pub fn new(default_mode: Mode) -> Self {
        Layout {
            workspaces: HashMap::new(),
            default_mode,
        }
    }

    pub fn get_focused_workspace(&mut self) -> &mut MiriWorkspace {
        self.workspaces
            .values_mut()
            .find(|workspace| workspace.focused)
            .expect("Could not get focused workspace")
    }

    pub fn set_focused_workspace_mode(&mut self, mode: Mode) {
        self.get_focused_workspace().mode = mode;
    }
}

#[derive(Debug)]
pub struct MiriWorkspace {
    pub focused: bool,
    pub mode: Mode,
    pub windows: Vec<MiriWindow>,
}

impl MiriWorkspace {
    pub fn get_focused_window(&self) -> &MiriWindow {
        self.windows
            .iter()
            .find(|window| window.is_focused)
            .expect("Could not get focused window")
    }
}

#[derive(Debug)]
pub struct MiriWindow {
    pub id: u64,
    pub position: (usize, usize),
    pub is_focused: bool,
    pub is_floating: bool,
}

pub fn copy_event_state_to_layout(event_state: &EventStreamState, layout: &mut Layout) {
    layout.workspaces.clear();
    for workspace in event_state.workspaces.workspaces.values() {
        let output_name = workspace
            .output
            .as_ref()
            .expect("Could not get workspace output")
            .clone();
        let key = (output_name, workspace.idx);

        let windows: Vec<MiriWindow> = event_state
            .windows
            .windows
            .values()
            .filter(|window| window.workspace_id == Some(workspace.id))
            .map(|window| {
                let position = window
                    .layout
                    .pos_in_scrolling_layout
                    .expect("Could not get position in scrolling layout");

                MiriWindow {
                    id: window.id,
                    position,
                    is_focused: window.is_focused,
                    is_floating: window.is_floating,
                }
            })
            .collect();

        let miri_workspace = MiriWorkspace {
            focused: workspace.is_focused,
            mode: layout.default_mode, // FIXME: get the actual mode here
            windows,
        };

        layout.workspaces.insert(key, miri_workspace);
    }
}
