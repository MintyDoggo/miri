use niri_ipc::{Action, Request, SizeChange, Window, socket::Socket};

use crate::{
    config::MiriConfig,
    ipc::Mode,
    layout::master::{handle_master_gain_window, handle_master_lose_window},
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
                if config.master_maximize_single_window {
                    let window = windows[0];
                    let action = Action::SetWindowWidth {
                        id: Some(window.id),
                        change: SizeChange::SetProportion(100.0),
                    };
                    socket
                        .send(Request::Action(action))
                        .expect("lost connection to niri")
                        .expect("niri rejected SetWindowWidth for single window");
                }
                return;
            }

            // handle master column
            socket
                .send(Request::Action(Action::MoveColumnToFirst {}))
                .expect("lost connection to niri")
                .expect("niri rejected MoveColumnToFirst");

            socket
                .send(Request::Action(Action::ConsumeOrExpelWindowLeft { id: None }))
                .expect("lost connection to niri")
                .expect("niri rejected ConsumeOrExpelWindowLeft");

            socket
                .send(Request::Action(Action::SetColumnWidth {
                    change: SizeChange::SetProportion(config.master_column_default_width_percentage),
                }))
                .expect("lost connection to niri")
                .expect("niri rejected SetColumnWidth for master column");

            // handle child column
            socket
                .send(Request::Action(Action::FocusColumnRight {}))
                .expect("lost connection to niri")
                .expect("niri rejected FocusColumnRight");

            socket
                .send(Request::Action(Action::SetColumnWidth {
                    change: SizeChange::SetProportion(100.0 - config.master_column_default_width_percentage),
                }))
                .expect("lost connection to niri")
                .expect("niri rejected SetColumnWidth for child column");

            for _ in 1..window_count {
                socket
                    .send(Request::Action(Action::ConsumeWindowIntoColumn {}))
                    .expect("lost connection to niri")
                    .expect("niri rejected ConsumeWindowIntoColumn");
            }

            socket
                .send(Request::Action(Action::FocusColumnLeft {}))
                .expect("lost connection to niri")
                .expect("niri rejected FocusColumnLeft");
        }
        Mode::Scroll => {}
    }
}
