use niri_ipc::state::EventStreamState;
use std::collections::HashMap;

use crate::{config::MiriConfig, ipc::Mode, niri_ipc_utils::get_focused_window};

pub struct ServiceState {
    pub previous_layout: Layout,
    pub current_layout: Layout,
    pub config: MiriConfig,
    pub first_niri_event_received: bool,
}

impl ServiceState {
    pub fn new(config: MiriConfig) -> Self {
        ServiceState {
            previous_layout: Layout::new(config.global.default_workspace_mode),
            current_layout: Layout::new(config.global.default_workspace_mode),
            config,
            first_niri_event_received: false,
        }
    }

    // determines if the window was spawned, or just simply moved/changed
    pub fn window_is_new(&self, window_id: &u64) -> bool {
        let previous_workspace = self
            .previous_layout
            .get_focused_workspace()
            .expect("Could not get previous focused workspace");
        let current_workspace = self
            .current_layout
            .get_focused_workspace()
            .expect("Could not get current focused workspace");

        // check if we moved to a new workspace
        if previous_workspace.id != current_workspace.id {
            return false;
        }

        match previous_workspace.windows.iter().find(|window| window.id == *window_id) {
            Some(_) => return false,
            None => return true,
        };
    }

    // sets workspace layout to scroll if there are already windows on the workspace. meant to be used on service start/restart
    pub fn initialize_workspace_modes(&mut self) {
        for workspace in self.previous_layout.workspaces.values_mut() {
            if workspace.windows.len() > 0 {
                workspace.mode = Mode::Scroll;
            }
        }
        for workspace in self.current_layout.workspaces.values_mut() {
            if workspace.windows.len() > 0 {
                workspace.mode = Mode::Scroll;
            }
        }
    }
}
#[derive(Debug)]
pub struct Layout {
    // TODO: make the key configurable to use (output, index) for people who want to assign the mode in this way
    pub workspaces: HashMap<u64, MiriWorkspace>,
    pub default_mode: Mode,
}

impl Layout {
    pub fn new(default_mode: Mode) -> Self {
        Layout {
            workspaces: HashMap::new(),
            default_mode,
        }
    }

    pub fn get_focused_workspace(&self) -> Option<&MiriWorkspace> {
        self.workspaces.values().find(|workspace| workspace.is_focused)
    }

    pub fn get_focused_workspace_mut(&mut self) -> Option<&mut MiriWorkspace> {
        self.workspaces.values_mut().find(|workspace| workspace.is_focused)
    }

    pub fn set_focused_workspace_mode(&mut self, mode: Mode) {
        let focused_workspace = self
            .get_focused_workspace_mut()
            .expect("Could not get focused workspace when attempting to set mode");

        focused_workspace.mode = mode;
    }
}

#[derive(Debug)]
pub struct MiriWorkspace {
    pub id: u64,
    pub output: String,
    pub index: u8,
    pub is_focused: bool,
    pub is_active: bool,
    pub mode: Mode,
    pub windows: Vec<MiriWindow>,
}

impl MiriWorkspace {
    pub fn get_focused_window(&self) -> Option<&MiriWindow> {
        self.windows.iter().find(|window| window.is_focused)
    }

    // crazy bit shift function to find the number of columns in O(N) time
    pub fn get_workspace_column_count(&self) -> i32 {
        let mut seen_columns = 0u64;
        let mut column_count = 0;
        for window in self.windows.iter() {
            // dont count floating windows
            let Some(position) = window.position else {
                continue;
            };

            debug_assert!(position.0 < 64, "column index exceeds bitmask capacity");
            let column_bit = 1u64 << position.0;
            if seen_columns & column_bit == 0 {
                seen_columns |= column_bit;
                column_count += 1;
            }
        }
        column_count
    }
}

#[derive(Debug)]
pub struct MiriWindow {
    pub id: u64,
    pub position: Option<(usize, usize)>,
    pub is_focused: bool,
    pub is_floating: bool,
}

// TODO: this function really needs to be rethought
pub fn copy_event_state_to_layout(event_state: &EventStreamState, previous_layout: &Layout, layout: &mut Layout) {
    layout.workspaces.clear();
    let mut focused_workspace_id: Option<u64> = None;

    // FIXME: this big loop is unnecessary and does not catch the case where a window is focused but not assigned to a workspace (dragging a window)
    for workspace in event_state.workspaces.workspaces.values() {
        let output_name = workspace
            .output
            .as_ref()
            .expect("Could not get workspace output")
            .clone();
        let key = workspace.id;

        let windows: Vec<MiriWindow> = event_state
            .windows
            .windows
            .values()
            .filter(|window| window.workspace_id == Some(workspace.id))
            .map(|window| MiriWindow {
                id: window.id,
                position: window.layout.pos_in_scrolling_layout,
                is_focused: window.is_focused,
                is_floating: window.is_floating,
            })
            .collect();

        let previous_mode = if let Some(previous_workspace) = previous_layout.workspaces.get(&workspace.id) {
            previous_workspace.mode
        } else {
            layout.default_mode
        };

        if workspace.is_focused {
            focused_workspace_id = Some(workspace.id);
        }

        let miri_workspace = MiriWorkspace {
            id: workspace.id,
            output: output_name.clone(),
            index: workspace.idx,
            is_focused: workspace.is_focused,
            is_active: workspace.is_active,
            mode: previous_mode,
            windows,
        };

        layout.workspaces.insert(key, miri_workspace);
    }

    // // while windows are being dragged around, event_state shows them as focused but assigned to no workspace
    // // for the purpose of our program, we want it to be assigned to the current focused workspace
    let focused_window_has_workspace = if let Some(focused_window) = get_focused_window(event_state) {
        focused_window.workspace_id.is_some()
    } else {
        false
    };

    if focused_window_has_workspace {
        // force the focused workspace to be where the currently focused window is
        for workspace in layout.workspaces.values_mut() {
            let has_focused_window = workspace.get_focused_window().is_some();

            if !has_focused_window && workspace.is_focused {
                workspace.is_focused = false;

                if Some(workspace.id) == focused_workspace_id {
                    focused_workspace_id = None;
                }
            }
            if has_focused_window && !workspace.is_focused {
                workspace.is_focused = true;
                focused_workspace_id = Some(workspace.id);
            }
        }

        // if we didnt have any focused window from the previous step, just set the focused workspace to the active workspace. FIXME: there can be multiple active workspaces at once with multiple monitors, so this is unpredictable
        if focused_workspace_id.is_none() {
            if let Some(active_workspace) = layout.workspaces.values_mut().find(|workspace| workspace.is_active) {
                active_workspace.is_focused = true;
            }
        }
    }
}
