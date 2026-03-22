use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response, Window, Workspace, state::EventStreamState};

pub const SUPPORTED_NIRI_VERSION: &str = "25.11";

pub fn get_niri_version(action_socket: &mut Socket) -> Option<String> {
    let reply = action_socket.send(Request::Version).ok()?;
    let response = reply.ok()?;
    match response {
        Response::Version(version) => Some(version),
        _ => None,
    }
}

pub fn warn_if_version_mismatch(action_socket: &mut Socket) {
    let Some(running_version) = get_niri_version(action_socket) else {
        eprintln!("[Warning]: Could not determine niri version");
        return;
    };
    let running_version_base = match running_version.split_whitespace().next() {
        Some(base) => base,
        None => {
            eprintln!("[Warning]: Could not parse niri version string");
            return;
        }
    };
    if running_version_base != SUPPORTED_NIRI_VERSION {
        eprintln!(
            "[Warning]: niri version mismatch! Expected {}, but running {}",
            SUPPORTED_NIRI_VERSION, running_version
        );
    }
}

pub fn get_focused_window(event_state: &EventStreamState) -> Option<&Window> {
    event_state.windows.windows.values().find(|window| window.is_focused)
}

fn get_focused_workspace(event_state: &EventStreamState) -> Option<&Workspace> {
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

pub fn get_focused_window_id(action_socket: &mut Socket) -> Option<u64> {
    let reply = action_socket.send(Request::FocusedWindow).ok()?;
    let response = reply.ok()?;
    match response {
        Response::FocusedWindow(Some(window)) => Some(window.id),
        _ => None,
    }
}
