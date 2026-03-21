use niri_ipc::{Action, Request, Window, socket::Socket};

use crate::{
    config::MiriConfig,
    service_state::{MiriWindow, MiriWorkspace},
};
// TODO: handle action result types in this file

fn handle_single_window(config: &MiriConfig, single_window_id: u64, action_socket: &mut Socket) {
    if config.master_maximize_single_window {
        println!("[DEBUG]: handling single window: {}", single_window_id);
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
    let previous_window_count = window_count - 1;
    let master_window_count = 1;
    let child_column_count = previous_window_count - master_window_count;

    let focus_action = Action::FocusWindow { id: window_to_move.id };
    action_socket
        .send(Request::Action(focus_action))
        .expect("lost connection to niri")
        .expect("niri rejected FocusWindow");

    // example: 4 windows in child column, focused window is at position 2 (1 based indexing). 4 - 2 = 2, move window up twice to be directly under the focused window
    let moves_needed = child_column_count.saturating_sub(focused_window.position.1);

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
    let current_windows = &current_workspace.windows;
    let window_count = current_windows.len();

    if window_count == 1 {
        println!("only 1!!!!");
        handle_single_window(&config, new_window.id, action_socket);
        return;
    }

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

    action_socket
        .send(Request::Action(move_into_child_column))
        .expect("lost connection to niri")
        .expect("niri rejected ConsumeOrExpelWindow");

    // if the new window went to the right of the child column, move it under our focused window. only do this for window open events
    if let Some(previous_focused_window) = previous_focused_window {
        let third_column_index = 3;
        if window_x >= third_column_index {
            move_window_under_focused_window(previous_focused_window, window_count, action_socket, new_window);
        }
    }

    let set_child_column_width = Action::SetWindowWidth {
        id: Some(new_window.id),
        change: niri_ipc::SizeChange::SetProportion(100.0 - config.master_column_default_width_percentage),
    };

    action_socket
        .send(Request::Action(set_child_column_width))
        .expect("lost connection to niri")
        .expect("niri rejected SetWindowWidth for child column");

    let master_window = current_windows
        .iter()
        .find(|window| window.position.0 == 1 && window.position.1 == 1)
        .expect("Could not find the master window when adding a new window");

    let set_master_proportion = Action::SetWindowWidth {
        id: Some(master_window.id),
        change: niri_ipc::SizeChange::SetProportion(config.master_column_default_width_percentage),
    };

    println!("{:?}", set_master_proportion);
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
    let current_windows = &current_workspace_state.windows;
    if current_windows.len() <= 0 {
        return;
    };
    if current_windows.len() == 1 {
        if let Some(window) = current_windows.first() {
            handle_single_window(config, window.id, action_socket);
        }
        return;
    }

    if current_windows.len() >= 2 {
        let master_closed: bool = current_workspace_state.get_workspace_column_count() == 1;

        if master_closed {
            let Some(top_child_window) = current_windows.iter().find(|&window| window.position.1 == 1) else {
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
