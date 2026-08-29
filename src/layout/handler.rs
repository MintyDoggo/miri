use niri_ipc::{Action, SizeChange, Window, socket::Socket};

use crate::{
    config::MiriConfig,
    ipc::Mode,
    layout::{
        master::{handle_master_gain_window, handle_master_lose_window},
        scroll::force_scroll_layout,
    },
    niri_ipc_utils::send_action,
    service_state::{MiriWindow, MiriWorkspace},
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
        Mode::Master => {
            let window_count = windows.len();

            if window_count == 0 {
                return;
            }

            if window_count == 1 {
                if config.master.maximize_single_window {
                    send_action(
                        socket,
                        Action::SetWindowWidth {
                            id: Some(windows[0].id),
                            change: SizeChange::SetProportion(100.0),
                        },
                    );
                }
                return;
            }

            // handle master column
            send_action(socket, Action::MoveColumnToFirst {});
            send_action(socket, Action::ConsumeOrExpelWindowLeft { id: None });
            send_action(
                socket,
                Action::SetColumnWidth {
                    change: SizeChange::SetProportion(config.master.column_width_percentage),
                },
            );

            // handle child column
            send_action(socket, Action::FocusColumnRight {});
            send_action(
                socket,
                Action::SetColumnWidth {
                    change: SizeChange::SetProportion(100.0 - config.master.column_width_percentage),
                },
            );

            for _ in 1..window_count {
                send_action(socket, Action::ConsumeWindowIntoColumn {});
            }

            send_action(socket, Action::FocusColumnLeft {});
        }
        Mode::Scroll => {
            if config.scroll.spread_windows_on_enter {
                force_scroll_layout(windows, socket, config.scroll.column_width_percentage);
            }
        }
    }
}
