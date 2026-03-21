use niri_ipc::{Action, Request, Window, socket::Socket};

use crate::service_state::ServiceState;

pub fn handle_scroll_window_open(service_state: &ServiceState, new_window: &Window, action_socket: &mut Socket) {
    if new_window.is_floating {
        return;
    }
    if service_state.config.scroll.maintain_focus_on_new_window {
        let focus_left = Action::FocusColumnLeft {};
        action_socket
            .send(Request::Action(focus_left))
            .expect("lost connection to niri")
            .expect("niri rejected FocusColumnLeft");
    }
}
