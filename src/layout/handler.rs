use niri_ipc::{Action, Request, SizeChange, Window, socket::Socket};
use std::collections::BTreeMap;

use crate::{
    config::MiriConfig,
    ipc::Mode,
    layout::master::{handle_master_gain_window, handle_master_lose_window},
    service_state::{MiriWindow, MiriWorkspace},
};

fn group_workspace_windows_by_column(workspace_windows: &[&Window]) -> BTreeMap<usize, Vec<(usize, u64)>> {
    let mut columns: BTreeMap<usize, Vec<(usize, u64)>> = BTreeMap::new();

    for window in workspace_windows {
        if window.is_floating {
            continue;
        }
        let Some((column, row)) = window.layout.pos_in_scrolling_layout else {
            continue;
        };

        columns.entry(column).or_default().push((row, window.id));
    }

    for windows_in_column in columns.values_mut() {
        windows_in_column.sort_unstable_by_key(|(row, _)| *row);
    }

    columns
}

fn force_scroll_layout(workspace_windows: Vec<&Window>, socket: &mut Socket, column_width_percentage: f64) {
    let focused_window_id = workspace_windows
        .iter()
        .find(|window| window.is_focused)
        .map(|window| window.id);
    let grouped_windows = group_workspace_windows_by_column(&workspace_windows);

    // Expel bottom windows one at a time from each original column.
    for windows_in_column in grouped_windows.values() {
        if let Some((_, anchor_window_id)) = windows_in_column.first().copied() {
            if windows_in_column.len() == 1 {
                continue;
            }
            socket
                .send(Request::Action(Action::FocusWindow { id: anchor_window_id }))
                .expect("lost connection to niri")
                .expect("niri rejected FocusWindow while spreading scroll columns");
            for _ in 1..windows_in_column.len() {
                socket
                    .send(Request::Action(Action::ExpelWindowFromColumn {}))
                    .expect("lost connection to niri")
                    .expect("niri rejected ExpelWindowFromColumn");
            }
        }
    }

    // Every tiled window is now its own full-height scrolling column.
    for window in workspace_windows
        .into_iter()
        .filter(|window| !window.is_floating && window.layout.pos_in_scrolling_layout.is_some())
    {
        socket
            .send(Request::Action(Action::SetWindowWidth {
                id: Some(window.id),
                change: SizeChange::SetProportion(column_width_percentage),
            }))
            .expect("lost connection to niri")
            .expect("niri rejected SetWindowWidth for scroll column");
        socket
            .send(Request::Action(Action::ResetWindowHeight { id: Some(window.id) }))
            .expect("lost connection to niri")
            .expect("niri rejected ResetWindowHeight for scroll window");
    }

    if let Some(id) = focused_window_id {
        socket
            .send(Request::Action(Action::FocusWindow { id }))
            .expect("lost connection to niri")
            .expect("niri rejected FocusWindow while restoring focus");
    }
}

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
                    change: SizeChange::SetProportion(config.master.column_width_percentage),
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
                    change: SizeChange::SetProportion(100.0 - config.master.column_width_percentage),
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
        Mode::Scroll => {
            if config.scroll.spread_windows_on_enter {
                force_scroll_layout(windows, socket, config.scroll.column_width_percentage);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use niri_ipc::WindowLayout;

    use super::*;

    fn window(id: u64, position: Option<(usize, usize)>, is_focused: bool, is_floating: bool) -> Window {
        Window {
            id,
            title: None,
            app_id: None,
            pid: None,
            workspace_id: Some(1),
            is_focused,
            is_floating,
            is_urgent: false,
            layout: WindowLayout {
                pos_in_scrolling_layout: position,
                tile_size: (0.0, 0.0),
                window_size: (0, 0),
                tile_pos_in_workspace_view: None,
                window_offset_in_tile: (0.0, 0.0),
            },
            focus_timestamp: None,
        }
    }

    #[test]
    fn groups_tiled_workspace_windows_by_column() {
        let top = window(10, Some((2, 1)), false, false);
        let bottom = window(11, Some((2, 2)), true, false);
        let first = window(12, Some((1, 1)), false, false);
        let floating = window(13, None, false, true);
        let windows = vec![&bottom, &floating, &first, &top];

        let grouped_windows = group_workspace_windows_by_column(&windows);

        assert_eq!(grouped_windows.get(&1), Some(&vec![(1, 12)]));
        assert_eq!(grouped_windows.get(&2), Some(&vec![(1, 10), (2, 11)]));
    }
}
