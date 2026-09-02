use niri_ipc::{Window, socket::Socket};

use crate::{
    config::MiriConfig,
    layout::{
        master::{force_master_layout, handle_master_gain_window, handle_master_lose_window},
        scroll::force_scroll_layout,
    },
    service_state::{MiriWindow, MiriWorkspace, Mode},
};

pub fn handle_workspace_gain_window(
    current_workspace: &MiriWorkspace,
    new_window: &Window,
    config: &MiriConfig,
    action_socket: &mut Socket,
    previous_focused_window: Option<&MiriWindow>,
) {
    if new_window.is_floating {
        return;
    }

    match current_workspace.mode {
        Mode::Master => {
            handle_master_gain_window(
                current_workspace,
                new_window,
                config,
                action_socket,
                previous_focused_window,
            );
        }
        Mode::Scroll => {}
    }
}

pub fn handle_workspace_lose_window(
    current_workspace_state: &MiriWorkspace,
    config: &MiriConfig,
    action_socket: &mut Socket,
) {
    match current_workspace_state.mode {
        Mode::Master => {
            handle_master_lose_window(current_workspace_state, config, action_socket);
        }
        Mode::Scroll => {}
    }
}

pub fn force_workspace_windows_into_layout_mode(
    windows: Vec<&Window>,
    socket: &mut Socket,
    config: &MiriConfig,
    mode: Mode,
) {
    match mode {
        Mode::Master => force_master_layout(windows, socket, config),
        Mode::Scroll => {
            if config.scroll.spread_windows_on_enter {
                force_scroll_layout(windows, socket, config.scroll.column_width_percentage);
            }
        }
    }
}
