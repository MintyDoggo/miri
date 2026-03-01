use common::{Mode, config::MiriConfig};
use niri_ipc::{Action, Request, SizeChange, Window, socket::Socket};

use crate::service_state::ServiceState;

// TODO: handle action result types

// FIXME: expect in here is really not a good pattern. we don't want this program to crash just because we were unable to make a window fullscreen for example. (or do we?)
pub fn handle_master_window_open(service_state: &mut ServiceState, new_window: &Window, action_socket: &mut Socket) {
    if new_window.is_floating {
        return;
    }

    let previous_windows = &service_state.previous_layout.get_focused_workspace().windows;
    let current_windows = &service_state.current_layout.get_focused_workspace().windows;

    let window_count = current_windows.len();

    if window_count == 1 {
        if service_state.config.master_maximize_single_window {
            println!("only 1!!!!");

            let full_screen_action = Action::SetWindowWidth {
                id: Some(new_window.id),
                change: niri_ipc::SizeChange::SetProportion(100.0),
            };
            let _ = action_socket
                .send(Request::Action(full_screen_action))
                .expect("Could not make single window full width");
        }
        return;
    }

    let Some(leftmost_window) = previous_windows
        .iter()
        .find(|&window| window.position.0 == 1 && window.position.1 == 1)
    else {
        eprintln!("Could not get left most window of focused workspace");
        return;
    };

    let move_into_child_column = if leftmost_window.is_focused {
        Action::ConsumeOrExpelWindowRight {
            id: Some(new_window.id),
        }
    } else {
        Action::ConsumeOrExpelWindowLeft {
            id: Some(new_window.id),
        }
    };

    let _ = action_socket
        .send(Request::Action(move_into_child_column))
        .expect("Could move new window into child column");

    // if we are focusing the child column, move the new window directly under the focused window
    if !leftmost_window.is_focused {
        let Some(focused_window) = previous_windows.iter().find(|window| window.is_focused) else {
            eprintln!("Could not find focused window");
            return;
        };

        let previous_window_count = window_count - 1;
        let master_window_count = 1;
        let child_column_count = previous_window_count - master_window_count;

        let focus_action = Action::FocusWindow { id: new_window.id };
        let _ = action_socket
            .send(Request::Action(focus_action))
            .expect("Could not focus new window");

        // example: 4 windows in child column, focused window is at position 2 (1 based indexing). 4 - 2 = 2, move window up twice to be directly under the focused window
        let moves_needed = child_column_count.saturating_sub(focused_window.position.1);

        for _ in 0..moves_needed {
            let _ = action_socket
                .send(Request::Action(Action::MoveWindowUp {}))
                .expect("Could not move window up");
        }
    }

    let set_master_proportion = Action::SetWindowWidth {
        id: Some(leftmost_window.id),
        change: niri_ipc::SizeChange::SetProportion(service_state.config.master_column_default_width_percentage),
    };

    let _ = action_socket
        .send(Request::Action(set_master_proportion))
        .expect("Could set master proportion");

    let set_child_column_width = Action::SetWindowWidth {
        id: Some(new_window.id),
        change: niri_ipc::SizeChange::SetProportion(
            100.0 - service_state.config.master_column_default_width_percentage,
        ),
    };

    let _ = action_socket
        .send(Request::Action(set_child_column_width))
        .expect("Could set master proportion");
}

pub fn handle_master_window_close(_closed_id: u64, service_state: &mut ServiceState, action_socket: &mut Socket) {
    let current_windows = &service_state.current_layout.get_focused_workspace().windows;
    if current_windows.len() <= 0 {
        return;
    };

    if current_windows.len() == 1 {
        println!("only 1!!!!");
        let Some(last_window) = current_windows.first() else {
            eprintln!("Getting left-most window returned none");
            return;
        };

        if service_state.config.master_maximize_single_window {
            let full_screen_action = Action::SetWindowWidth {
                id: Some(last_window.id),
                change: niri_ipc::SizeChange::SetProportion(100.0),
            };
            let _ = action_socket
                .send(Request::Action(full_screen_action))
                .expect("Could not make single window full width");
        }
    }

    if current_windows.len() >= 2 {
        // this is a workaround: basically, sometimes previous_state can contain 2 windows on the same workspace with postion `(1, 1)`.
        // When this happens, its impossible to determine if the window that was closed was the master window (the window that has position 1, 1. There are 2)
        // note that this is likely not a problem with my code, this is just what happens when you use `event_state.apply(event.clone())` before matching events.
        // the workaround I use is to check how many columns there are in this workspace. we can do this with the following line of code
        // if there is a window with an x position of 2 or greater, it means that the master window was NOT closed.
        let master_closed: bool = !current_windows.iter().find(|window| window.position.0 >= 2).is_some();

        if master_closed {
            let Some(top_child_window) = current_windows.iter().find(|&window| window.position.1 == 1) else {
                eprintln!("Could not find top window in child column");
                return;
            };

            let expel_action = Action::ConsumeOrExpelWindowLeft {
                id: Some(top_child_window.id),
            };
            let _ = action_socket
                .send(Request::Action(expel_action))
                .expect("Could not expel child window left");

            let focus_action = Action::FocusColumnLeft {};
            let _ = action_socket
                .send(Request::Action(focus_action))
                .expect("Could focus left column");
        }
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
