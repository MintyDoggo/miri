use niri_ipc::{Action, WorkspaceReferenceArg};

use crate::ipc::MiriOverride;
use crate::niri_ipc_utils::{get_focused_window_id, send_action};
use crate::service_state::{Mode, ServiceState};
use niri_ipc::socket::Socket;

fn passthrough_action(action: Action, action_socket: &mut Socket) {
    send_action(action_socket, action);
}

pub fn scroll_passthrough(override_action: MiriOverride, action_socket: &mut Socket) {
    match override_action {
        MiriOverride::MoveColumnLeft => {
            passthrough_action(Action::MoveColumnLeft {}, action_socket);
        }
        MiriOverride::MoveColumnRight => {
            passthrough_action(Action::MoveColumnRight {}, action_socket);
        }
        MiriOverride::MoveColumnToFirst => {
            passthrough_action(Action::MoveColumnToFirst {}, action_socket);
        }
        MiriOverride::MoveColumnToLast => {
            passthrough_action(Action::MoveColumnToLast {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorUp => {
            passthrough_action(Action::MoveColumnToMonitorUp {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorDown => {
            passthrough_action(Action::MoveColumnToMonitorDown {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorLeft => {
            passthrough_action(Action::MoveColumnToMonitorLeft {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorRight => {
            passthrough_action(Action::MoveColumnToMonitorRight {}, action_socket);
        }
        MiriOverride::MoveColumnToWorkspaceUp => {
            passthrough_action(Action::MoveColumnToWorkspaceUp { focus: true }, action_socket);
        }
        MiriOverride::MoveColumnToWorkspaceDown => {
            passthrough_action(Action::MoveColumnToWorkspaceDown { focus: true }, action_socket);
        }
        MiriOverride::MoveColumnToWorkspace { index } => {
            passthrough_action(
                Action::MoveColumnToWorkspace {
                    reference: WorkspaceReferenceArg::Index(index),
                    focus: true,
                },
                action_socket,
            );
        }
    }
}

fn master_override(override_action: MiriOverride, action_socket: &mut Socket) {
    match override_action {
        MiriOverride::MoveColumnLeft => {
            passthrough_action(Action::SwapWindowLeft {}, action_socket);
        }
        MiriOverride::MoveColumnRight => {
            passthrough_action(Action::SwapWindowRight {}, action_socket);
        }
        MiriOverride::MoveColumnToFirst => {
            passthrough_action(Action::SwapWindowLeft {}, action_socket);
        }
        MiriOverride::MoveColumnToLast => {
            passthrough_action(Action::SwapWindowRight {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorUp => {
            passthrough_action(Action::MoveWindowToMonitorUp {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorDown => {
            passthrough_action(Action::MoveWindowToMonitorDown {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorLeft => {
            passthrough_action(Action::MoveWindowToMonitorLeft {}, action_socket);
        }
        MiriOverride::MoveColumnToMonitorRight => {
            passthrough_action(Action::MoveWindowToMonitorRight {}, action_socket);
        }
        MiriOverride::MoveColumnToWorkspaceUp => {
            passthrough_action(Action::MoveWindowToWorkspaceUp { focus: true }, action_socket);
        }
        MiriOverride::MoveColumnToWorkspaceDown => {
            passthrough_action(Action::MoveWindowToWorkspaceDown { focus: true }, action_socket);
        }
        MiriOverride::MoveColumnToWorkspace { index } => {
            if let Some(window_id) = get_focused_window_id(action_socket) {
                passthrough_action(
                    Action::MoveWindowToWorkspace {
                        window_id: Some(window_id),
                        reference: WorkspaceReferenceArg::Index(index),
                        focus: true,
                    },
                    action_socket,
                );
            }
        }
    }
}

pub fn handle_override(override_action: MiriOverride, action_socket: &mut Socket, service_state: &ServiceState) {
    let current_workspace = match service_state.current_layout.get_focused_workspace() {
        Some(ws) => ws,
        None => {
            eprintln!("Could not get focused workspace for override");
            return;
        }
    };

    match current_workspace.mode {
        Mode::Scroll => scroll_passthrough(override_action, action_socket),
        Mode::Master => master_override(override_action, action_socket),
    }
}
