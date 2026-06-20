use niri_ipc::{Action, Request, Window, socket::Socket};

use crate::{
    config::MiriConfig,
    service_state::{MiriWindow, MiriWorkspace},
};

fn handle_single_window(config: &MiriConfig, single_window_id: u64, action_socket: &mut Socket) {
    if config.master.maximize_single_window {
        let full_screen_action = Action::SetWindowWidth {
            id: Some(single_window_id),
            change: niri_ipc::SizeChange::SetProportion(100.0),
        };
        action_socket
            .send(Request::Action(full_screen_action))
            .expect("lost connection to niri")
            .expect("niri rejected SetWindowWidth");
    }
}

fn move_window_under_focused_window(
    focused_window: &MiriWindow,
    window_count: usize,
    action_socket: &mut Socket,
    window_to_move: &Window,
) {
    // if the focused window is floating, get out of here
    let Some(focused_window_position) = focused_window.position else {
        return;
    };

    let previous_window_count = window_count - 1;
    let master_window_count = 1;
    let child_column_count = previous_window_count - master_window_count;

    let focus_action = Action::FocusWindow { id: window_to_move.id };
    action_socket
        .send(Request::Action(focus_action))
        .expect("lost connection to niri")
        .expect("niri rejected FocusWindow");

    // example: 4 windows in child column, focused window is at position 2 (1 based indexing). 4 - 2 = 2, move window up twice to be directly under the focused window
    let moves_needed = child_column_count.saturating_sub(focused_window_position.1);

    for _ in 0..moves_needed {
        action_socket
            .send(Request::Action(Action::MoveWindowUp {}))
            .expect("lost connection to niri")
            .expect("niri rejected MoveWindowUp");
    }
}

pub fn handle_master_gain_window(
    current_workspace: &MiriWorkspace,
    new_window: &Window,
    config: &MiriConfig,
    action_socket: &mut Socket,
    previous_focused_window: Option<&MiriWindow>,
) {
    if new_window.is_floating {
        return;
    }

    let tiled_windows: Vec<&MiriWindow> = current_workspace
        .windows
        .iter()
        .filter(|window| !window.is_floating)
        .collect();

    if tiled_windows.is_empty() {
        return;
    };

    if tiled_windows.len() == 1 {
        handle_single_window(&config, new_window.id, action_socket);
        return;
    }

    let (window_x, _) = new_window
        .layout
        .pos_in_scrolling_layout
        .expect("Could not get position in scrolling layout of tiled window");

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

    action_socket
        .send(Request::Action(move_into_child_column))
        .expect("lost connection to niri")
        .expect("niri rejected ConsumeOrExpelWindow");

    // if the new window went to the right of the child column, move it under our focused window. only do this for window open events
    if let Some(previous_focused_window) = previous_focused_window {
        let third_column_index = 3;
        if window_x >= third_column_index {
            move_window_under_focused_window(previous_focused_window, tiled_windows.len(), action_socket, new_window);
        }
    }

    let set_child_column_width = Action::SetWindowWidth {
        id: Some(new_window.id),
        change: niri_ipc::SizeChange::SetProportion(100.0 - config.master.column_width_percentage),
    };

    action_socket
        .send(Request::Action(set_child_column_width))
        .expect("lost connection to niri")
        .expect("niri rejected SetWindowWidth for child column");

    let master_window = tiled_windows
        .iter()
        .find(|window| window.position == Some((1, 1)))
        .expect("Could not find the master window when adding a new window");

    let set_master_proportion = Action::SetWindowWidth {
        id: Some(master_window.id),
        change: niri_ipc::SizeChange::SetProportion(config.master.column_width_percentage),
    };

    action_socket
        .send(Request::Action(set_master_proportion))
        .expect("lost connection to niri")
        .expect("niri rejected SetWindowWidth for master column");
}

pub fn handle_master_lose_window(
    current_workspace_state: &MiriWorkspace,
    config: &MiriConfig,
    action_socket: &mut Socket,
) {
    let tiled_windows: Vec<&MiriWindow> = current_workspace_state
        .windows
        .iter()
        .filter(|window| !window.is_floating)
        .collect();

    if tiled_windows.is_empty() {
        return;
    };

    if tiled_windows.len() == 1 {
        handle_single_window(config, tiled_windows[0].id, action_socket);
        return;
    }

    if tiled_windows.len() >= 2 {
        let master_closed: bool = current_workspace_state.get_workspace_column_count() == 1;

        if master_closed {
            let Some(top_child_window) = tiled_windows
                .iter()
                .find(|window| window.position.map(|(_, row)| row == 1).unwrap_or(false))
            else {
                eprintln!("Could not find top window in child column");
                return;
            };

            let expel_action = Action::ConsumeOrExpelWindowLeft {
                id: Some(top_child_window.id),
            };
            action_socket
                .send(Request::Action(expel_action))
                .expect("lost connection to niri")
                .expect("niri rejected ConsumeOrExpelWindowLeft");

            let focus_action = Action::FocusColumnLeft {};
            action_socket
                .send(Request::Action(focus_action))
                .expect("lost connection to niri")
                .expect("niri rejected FocusColumnLeft");
        }
    }
}
