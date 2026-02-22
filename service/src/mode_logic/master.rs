use std::iter;

use common::{Mode, config::MiriConfig};
use niri_ipc::{Action, Request, SizeChange, Window, socket::Socket, state::EventStreamState};

use crate::{niri_ipc_utils::get_windows_on_focused_workspace, service_state::ServiceState};

// FIXME: expect in here is really not a good pattern. we don't want this program to crash just because we were unable to make a window fullscreen for example. (or do we?)
pub fn handle_master_window_open(
    service_state: &ServiceState,
    new_window: &Window,
    event_state: &EventStreamState,
    action_socket: &mut Socket,
) {
    let Some(windows) = get_windows_on_focused_workspace(event_state) else {
        eprintln!("Could not get windows on focused workspace");
        return;
    };
    let window_count = windows.len() + 1;

    // FIXME: need to see if this is performant or not
    let mut all_windows = windows.iter().copied().chain(iter::once(new_window));

    if window_count == 1 {
        if service_state.config.master_maximize_single_window {
            println!("only 1!!!!");

            let full_screen_action = Action::SetWindowWidth {
                id: Some(new_window.id),
                change: niri_ipc::SizeChange::SetProportion(100.0),
            };
            action_socket
                .send(Request::Action(full_screen_action))
                .expect("Could not make single window full width")
                .expect("msg");
        }
        return;
    }

    let Some(leftmost_window) =
        all_windows.find(|&window| window.layout.pos_in_scrolling_layout.map_or(false, |(x, _)| x == 1))
    else {
        eprintln!("Could not get left most window");
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

    action_socket
        .send(Request::Action(move_into_child_column))
        .expect("Could move new window into child column")
        .expect("msg");

    // if we are focusing the child column, move the new window directly under the focused window
    if !leftmost_window.is_focused {
        let Some(focused_window) = windows.iter().find(|w| w.is_focused) else {
            eprintln!("Could not find focused window");
            return;
        };

        let Some((_, focused_y)) = focused_window.layout.pos_in_scrolling_layout else {
            eprintln!("Focused window has no scrolling layout position");
            return;
        };

        // we can assume window count is itself - 2 since we have already checked if there are more than 1 windows. `-2` because we added the new window to this already. i dont like this line lol
        let child_column_count = window_count - 2;

        let focus_action = Action::FocusWindow { id: new_window.id };
        action_socket
            .send(Request::Action(focus_action))
            .expect("Could not focus new window")
            .expect("msg");

        // example: 4 windows in child column, focused window is at position 2 (1 based indexing). 4 - 2 = 2, move window up twice to be directly under the focused window
        let moves_needed = child_column_count.saturating_sub(focused_y);

        for _ in 0..moves_needed {
            action_socket
                .send(Request::Action(Action::MoveWindowUp {}))
                .expect("Could not move window up")
                .expect("msg");
        }
    }

    let set_master_proportion = Action::SetWindowWidth {
        id: Some(leftmost_window.id),
        change: niri_ipc::SizeChange::SetProportion(service_state.config.master_column_default_width_percentage),
    };

    action_socket
        .send(Request::Action(set_master_proportion))
        .expect("Could set master proportion")
        .expect("msg");
}

pub fn handle_master_window_close(
    service_state: &ServiceState,
    event_state: &EventStreamState,
    action_socket: &mut Socket,
) {
    let Some(windows) = get_windows_on_focused_workspace(event_state) else {
        // TODO: this is really not a great way of handling it. this basically means "we either couldnt get the focused workspace or there were no windows on this workspace"
        eprintln!("Could not get windows on focused workspace");
        return;
    };

    let window_count = windows.len() - 1;
    if window_count != 1 {
        return;
    }

    let Some(&last_window) = windows
        .iter()
        .find(|window| window.layout.pos_in_scrolling_layout.is_some_and(|(x, _)| x == 1))
    else {
        eprintln!("Getting left-most window returned none");
        return;
    };

    if service_state.config.master_maximize_single_window {
        println!("only 1!!!!");

        let full_screen_action = Action::SetWindowWidth {
            id: Some(last_window.id),
            change: niri_ipc::SizeChange::SetProportion(100.0),
        };
        action_socket
            .send(Request::Action(full_screen_action))
            .expect("Could not make single window full width")
            .expect("msg");
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
