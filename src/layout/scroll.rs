use std::collections::BTreeMap;

use niri_ipc::{Action, SizeChange, Window, socket::Socket};

use crate::{niri_ipc_utils::send_action, service_state::ServiceState};

// FIXME: this is unused, i dont really like this config option but we may as well provide it
pub fn handle_scroll_window_open(service_state: &ServiceState, new_window: &Window, action_socket: &mut Socket) {
    if new_window.is_floating {
        return;
    }
    if service_state.config.scroll.maintain_focus_on_new_window {
        send_action(action_socket, Action::FocusColumnLeft {});
    }
}

pub fn force_scroll_layout(workspace_windows: Vec<&Window>, socket: &mut Socket, column_width_percentage: f64) {
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

            send_action(socket, Action::FocusWindow { id: anchor_window_id });
            for _ in 1..windows_in_column.len() {
                send_action(socket, Action::ExpelWindowFromColumn {});
            }
        }
    }

    // Every tiled window is now its own full-height scrolling column.
    for window in workspace_windows
        .into_iter()
        .filter(|window| !window.is_floating && window.layout.pos_in_scrolling_layout.is_some())
    {
        send_action(
            socket,
            Action::SetWindowWidth {
                id: Some(window.id),
                change: SizeChange::SetProportion(column_width_percentage),
            },
        );

        send_action(socket, Action::ResetWindowHeight { id: Some(window.id) });
    }

    if let Some(id) = focused_window_id {
        send_action(socket, Action::FocusWindow { id });
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
