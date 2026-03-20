use niri_ipc::state::EventStreamState;
use std::collections::HashMap;

use crate::{config::MiriConfig, ipc::Mode};

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
    // FIXME: just make this map id to workspace
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
            debug_assert!(window.position.0 < 64, "column index exceeds bitmask capacity");
            let column_bit = 1u64 << window.position.0;
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
    pub position: (usize, usize),
    pub is_focused: bool,
    pub is_floating: bool,
}

pub fn copy_event_state_to_layout(event_state: &EventStreamState, previous_layout: &Layout, layout: &mut Layout) {
    layout.workspaces.clear();
    let mut focused_workspace_id: Option<u64> = None;

    for workspace in event_state.workspaces.workspaces.values() {
        let output_name = workspace
            .output
            .as_ref()
            .expect("Could not get workspace output")
            .clone();
        let key = (output_name.clone(), workspace.idx);

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

        let previous_mode =
            if let Some(previous_workspace) = previous_layout.workspaces.get(&(output_name.clone(), workspace.idx)) {
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

    // force the focused workspace to be where the currently focused window is, or if there is none, the current active workspace
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

    if focused_workspace_id.is_none() {
        if let Some(active_workspace) = layout.workspaces.values_mut().find(|workspace| workspace.is_active) {
            active_workspace.is_focused = true;
        }
    }
}
