use niri_ipc::{Window, Workspace, state::EventStreamState};

use crate::service_state::ServiceState;

pub fn window_is_new(window_id: &u64, service_state: &mut ServiceState) -> bool {
    let previous_workspace = service_state.previous_layout.get_focused_workspace();
    let current_workspace = service_state.current_layout.get_focused_workspace();
    if previous_workspace.id != current_workspace.id {
        return false;
    }

    match previous_workspace.windows.iter().find(|window| window.id == *window_id) {
        Some(_) => return false,
        None => return true,
    };
}

pub fn get_focused_workspace(event_state: &EventStreamState) -> Option<&Workspace> {
    event_state
        .workspaces
        .workspaces
        .values()
        .find(|workspace| workspace.is_focused)
}

pub fn get_windows_on_focused_workspace(event_state: &EventStreamState) -> Option<Vec<&Window>> {
    let Some(focused_workspace) = get_focused_workspace(event_state) else {
        eprintln!("Could not get focused workspace");
        return None;
    };
    let workspace_windows: Vec<&Window> = event_state
        .windows
        .windows
        .iter()
        .filter(|(_, window)| window.workspace_id == Some(focused_workspace.id))
        .map(|(_, window)| window)
        .collect();

    Some(workspace_windows)
}
