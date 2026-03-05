use common::{Mode, config::MiriConfig};
use niri_ipc::{Action, Request, SizeChange, Window, socket::Socket};

use crate::service_state::{MiriWindow, ServiceState};

// TODO: handle action result types

fn handle_single_window(config: &MiriConfig, single_window_id: u64, action_socket: &mut Socket) {
    if config.master_maximize_single_window {
        println!("[DEBUG]: handling single window: {}", single_window_id);
        let full_screen_action = Action::SetWindowWidth {
            id: Some(single_window_id),
            change: niri_ipc::SizeChange::SetProportion(100.0),
        };
        if let Err(e) = action_socket
            .send(Request::Action(full_screen_action))
            .expect("Could not make single window full width")
        {
            eprint!("{e}");
        }
    }
}

fn move_window_under_focused_window(
    focused_window: &MiriWindow,
    window_count: usize,
    action_socket: &mut Socket,
    window_to_move: &Window,
) {
    let previous_window_count = window_count - 1;
    let master_window_count = 1;
    let child_column_count = previous_window_count - master_window_count;

    let focus_action = Action::FocusWindow { id: window_to_move.id };
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

// FIXME: expect in here is really not a good pattern. we don't want this program to crash just because we were unable to make a window fullscreen for example. (or do we?)
pub fn handle_master_window_open(service_state: &mut ServiceState, new_window: &Window, action_socket: &mut Socket) {
    if new_window.is_floating {
        return;
    }

    let previous_focused_workspace = service_state.previous_layout.get_focused_workspace();
    let current_focused_workspace = service_state.current_layout.get_focused_workspace();

    let workspace_changed = previous_focused_workspace.id != current_focused_workspace.id;
    let current_windows = &current_focused_workspace.windows;

    if workspace_changed {
        println!("[DEBUG]: CHANGE handling previous workspace");
        // TODO: move all this in its own function call `handle_from_workspace` or something
        let from_workspace = service_state
            .current_layout
            .workspaces
            .get(&(
                previous_focused_workspace.output.clone(),
                previous_focused_workspace.index,
            ))
            .expect("Could not get the from workspace for workspace changed");

        if from_workspace.windows.len() == 1 {
            let single_window = from_workspace
                .windows
                .first()
                .expect("Could not get the first window from the from workspace");
            println!("[DEBUG]: single window");

            handle_single_window(&service_state.config, single_window.id, action_socket);
        }
    }

    let window_count = current_windows.len();

    if window_count == 1 {
        println!("only 1!!!!");
        handle_single_window(&service_state.config, new_window.id, action_socket);
        return;
    }

    let Some(previous_master_window) = current_windows
        .iter()
        .find(|&window| window.position.0 == 1 && window.position.1 == 1)
    else {
        eprintln!("Could not get left most window of focused workspace");
        return;
    };
    let (window_x, _) = new_window
        .layout
        .pos_in_scrolling_layout
        .expect("Could not get position in scrolling layout");

    let move_into_child_column = match window_x {
        2 => Action::ConsumeOrExpelWindowRight {
            id: Some(new_window.id),
        },
        3.. => Action::ConsumeOrExpelWindowLeft {
            id: Some(new_window.id),
        },
        _ => {
            eprintln!(
                "Window X position was not valid when trying to adjust new window. x position was {}",
                window_x
            );
            return;
        }
    };

    let _ = action_socket
        .send(Request::Action(move_into_child_column))
        .expect("Could not move new window into child column");

    // if the new window went to the right of the child column, move it under our focused window. only do this for window open events
    if window_x >= 3 && !workspace_changed {
        let previous_focused_window = previous_focused_workspace
            .get_focused_window()
            .expect("Could not get focused window of previous focused workspace");

        move_window_under_focused_window(previous_focused_window, window_count, action_socket, new_window);
    }

    let set_child_column_width = Action::SetWindowWidth {
        id: Some(new_window.id),
        change: niri_ipc::SizeChange::SetProportion(
            100.0 - service_state.config.master_column_default_width_percentage,
        ),
    };

    let _ = action_socket
        .send(Request::Action(set_child_column_width))
        .expect("Could not set child proportion");

    let set_master_proportion = Action::SetWindowWidth {
        id: Some(previous_master_window.id),
        change: niri_ipc::SizeChange::SetProportion(service_state.config.master_column_default_width_percentage),
    };

    println!("{:?}", set_master_proportion);
    let _ = action_socket
        .send(Request::Action(set_master_proportion))
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
