use niri_ipc::{Action, Request, SizeChange, Window, socket::Socket};
use std::collections::BTreeMap;

use crate::{
    config::MiriConfig,
    ipc::Mode,
    layout::master::{handle_master_gain_window, handle_master_lose_window},
    service_state::{MiriWindow, MiriWorkspace},
};

#[derive(Debug, PartialEq)]
struct ScrollLayoutPlan {
    columns: BTreeMap<usize, Vec<(usize, u64)>>,
    window_ids: Vec<u64>,
    focused_window_id: Option<u64>,
}

fn plan_scroll_layout(windows: &[&Window]) -> ScrollLayoutPlan {
    let mut columns: BTreeMap<usize, Vec<(usize, u64)>> = BTreeMap::new();
    let mut window_ids = Vec::new();
    let mut focused_window_id = None;

    for window in windows {
        if window.is_floating {
            continue;
        }
        let Some((column, row)) = window.layout.pos_in_scrolling_layout else {
            continue;
        };

        columns.entry(column).or_default().push((row, window.id));
        window_ids.push(window.id);
        if window.is_focused {
            focused_window_id = Some(window.id);
        }
    }

    for windows_in_column in columns.values_mut() {
        windows_in_column.sort_unstable_by_key(|(row, _)| *row);
    }

    ScrollLayoutPlan {
        columns,
        window_ids,
        focused_window_id,
    }
}

fn force_scroll_layout(windows: Vec<&Window>, socket: &mut Socket, config: &MiriConfig) {
    if !config.scroll.spread_windows_on_enter {
        return;
    }

    let plan = plan_scroll_layout(&windows);

    // Expel bottom windows one at a time. Re-focus the top window before each
    // action because niri focuses the window it just expelled.
    for windows_in_column in plan.columns.values() {
        if let Some((_, anchor_window_id)) = windows_in_column.first().copied() {
            for _ in 1..windows_in_column.len() {
                socket
                    .send(Request::Action(Action::FocusWindow { id: anchor_window_id }))
                    .expect("lost connection to niri")
                    .expect("niri rejected FocusWindow while spreading scroll columns");
                socket
                    .send(Request::Action(Action::ExpelWindowFromColumn {}))
                    .expect("lost connection to niri")
                    .expect("niri rejected ExpelWindowFromColumn");
            }
        }
    }

    // Every tiled window is now its own full-height scrolling column.
    for window_id in plan.window_ids {
        socket
            .send(Request::Action(Action::SetWindowWidth {
                id: Some(window_id),
                change: SizeChange::SetProportion(config.scroll.column_width_percentage),
            }))
            .expect("lost connection to niri")
            .expect("niri rejected SetWindowWidth for scroll column");
        socket
            .send(Request::Action(Action::ResetWindowHeight { id: Some(window_id) }))
            .expect("lost connection to niri")
            .expect("niri rejected ResetWindowHeight for scroll window");
    }

    if let Some(id) = plan.focused_window_id {
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
        Mode::Scroll => force_scroll_layout(windows, socket, config),
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
    fn plans_tiled_windows_by_column_and_preserves_focus() {
        let top = window(10, Some((2, 1)), false, false);
        let bottom = window(11, Some((2, 2)), true, false);
        let first = window(12, Some((1, 1)), false, false);
        let floating = window(13, None, false, true);
        let windows = vec![&bottom, &floating, &first, &top];

        let plan = plan_scroll_layout(&windows);

        assert_eq!(plan.columns.get(&1), Some(&vec![(1, 12)]));
        assert_eq!(plan.columns.get(&2), Some(&vec![(1, 10), (2, 11)]));
        assert_eq!(plan.window_ids, vec![11, 12, 10]);
        assert_eq!(plan.focused_window_id, Some(11));
    }
}
