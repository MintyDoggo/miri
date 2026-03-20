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
                        .expect("Failed to maximize single window")
                        .expect("Failed to maximize single window response");
                }
                return;
            }

            // handle master column
            socket
                .send(Request::Action(Action::MoveColumnToFirst {}))
                .expect("Failed to move column to first")
                .expect("Failed to move column to first response");

            socket
                .send(Request::Action(Action::ConsumeOrExpelWindowLeft { id: None }))
                .expect("Failed to consume/expel window left")
                .expect("Failed to consume/expel window left response");

            socket
                .send(Request::Action(Action::SetColumnWidth {
                    change: SizeChange::SetProportion(config.master_column_default_width_percentage),
                }))
                .expect("Failed to set master column width")
                .expect("Failed to set master column width response");

            // handle child column
            socket
                .send(Request::Action(Action::FocusColumnRight {}))
                .expect("Failed to focus column right")
                .expect("Failed to focus column right response");

            socket
                .send(Request::Action(Action::SetColumnWidth {
                    change: SizeChange::SetProportion(100.0 - config.master_column_default_width_percentage),
                }))
                .expect("Failed to set secondary column width")
                .expect("Failed to set secondary column width response");

            for _ in 1..window_count {
                socket
                    .send(Request::Action(Action::ConsumeWindowIntoColumn {}))
                    .expect("Failed to consume window into column")
                    .expect("Failed to consume window into column response");
            }

            socket
                .send(Request::Action(Action::FocusColumnLeft {}))
                .expect("Failed to return focus to master")
                .expect("Failed to return focus to master response");
        }
        Mode::Scroll => {}
    }
}
