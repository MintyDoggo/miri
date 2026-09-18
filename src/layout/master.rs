use niri_ipc::{Action, SizeChange, Window, socket::Socket};

use crate::{
    config::MiriConfig,
    niri_ipc_utils::{get_output_logical_size, send_action},
    service_state::{MiriWindow, MiriWorkspace},
};

fn handle_single_window(config: &MiriConfig, single_window_id: u64, output: &str, action_socket: &mut Socket) {
    if config.master.maximize_single_window {
        let change = single_window_size_change(config, output, action_socket);
        send_action(
            action_socket,
            Action::SetWindowWidth {
                id: Some(single_window_id),
                change,
            },
        );
    }
}

fn single_window_size_change(config: &MiriConfig, output: &str, action_socket: &mut Socket) -> SizeChange {
    let full_width = SizeChange::SetProportion(100.0);

    if !config.master.limits_single_window() {
        return full_width;
    }

    let Some((output_width, output_height)) = get_output_logical_size(action_socket, output) else {
        eprintln!(
            "Could not get the logical size of output {}, using the full width",
            output
        );
        return full_width;
    };

    match config.master.single_window_width(output_width, output_height) {
        Some(width) => SizeChange::SetFixed(width),
        None => full_width,
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

    send_action(action_socket, Action::FocusWindow { id: window_to_move.id });

    // example: 4 windows in child column, focused window is at position 2 (1 based indexing). 4 - 2 = 2, move window up twice to be directly under the focused window
    let moves_needed = child_column_count.saturating_sub(focused_window_position.1);

    for _ in 0..moves_needed {
        send_action(action_socket, Action::MoveWindowUp {});
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
        handle_single_window(config, new_window.id, &current_workspace.output, action_socket);
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

    send_action(action_socket, move_into_child_column);

    // if the new window went to the right of the child column, move it under our focused window. only do this for window open events
    if let Some(previous_focused_window) = previous_focused_window {
        let third_column_index = 3;
        if window_x >= third_column_index {
            move_window_under_focused_window(previous_focused_window, tiled_windows.len(), action_socket, new_window);
        }
    }

    send_action(
        action_socket,
        Action::SetWindowWidth {
            id: Some(new_window.id),
            change: niri_ipc::SizeChange::SetProportion(100.0 - config.master.column_width_percentage),
        },
    );

    let master_window = tiled_windows
        .iter()
        .find(|window| window.position == Some((1, 1)))
        .expect("Could not find the master window when adding a new window");

    send_action(
        action_socket,
        Action::SetWindowWidth {
            id: Some(master_window.id),
            change: niri_ipc::SizeChange::SetProportion(config.master.column_width_percentage),
        },
    );
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
        handle_single_window(
            config,
            tiled_windows[0].id,
            &current_workspace_state.output,
            action_socket,
        );
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

            send_action(
                action_socket,
                Action::ConsumeOrExpelWindowLeft {
                    id: Some(top_child_window.id),
                },
            );

            send_action(action_socket, Action::FocusColumnLeft {});
        }
    }
}

pub fn force_master_layout(workspace_windows: Vec<&Window>, socket: &mut Socket, config: &MiriConfig, output: &str) {
    let window_count = workspace_windows.len();

    if window_count == 0 {
        return;
    }

    if window_count == 1 {
        handle_single_window(config, workspace_windows[0].id, output, socket);
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
