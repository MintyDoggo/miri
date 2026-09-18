use niri_ipc::{Window, socket::Socket};
pub mod master;
pub mod scroll;

use crate::{
    config::MiriConfig,
    layout::{
        master::{force_master_layout, handle_master_gain_window, handle_master_lose_window},
        scroll::force_scroll_layout,
    },
    service_state::{MiriWindow, MiriWorkspace, Mode},
};

pub trait WorkspaceLayout {
    fn gain_window(
        &self,
        new_window: &Window,
        config: &MiriConfig,
        action_socket: &mut Socket,
        previous_focused_window: Option<&MiriWindow>,
    );
    fn lose_window(&self, config: &MiriConfig, action_socket: &mut Socket);
    fn force_mode(&self, windows: Vec<&Window>, socket: &mut Socket, config: &MiriConfig);
}

impl WorkspaceLayout for MiriWorkspace {
    fn gain_window(
        &self,
        new_window: &Window,
        config: &MiriConfig,
        action_socket: &mut Socket,
        previous_focused_window: Option<&MiriWindow>,
    ) {
        if new_window.is_floating {
            return;
        }

        match self.mode {
            Mode::Master => {
                handle_master_gain_window(self, new_window, config, action_socket, previous_focused_window);
            }
            Mode::Scroll => {}
        }
    }

    fn lose_window(&self, config: &MiriConfig, action_socket: &mut Socket) {
        match self.mode {
            Mode::Master => {
                handle_master_lose_window(self, config, action_socket);
            }
            Mode::Scroll => {}
        }
    }
    fn force_mode(&self, windows: Vec<&Window>, socket: &mut Socket, config: &MiriConfig) {
        match self.mode {
            Mode::Master => force_master_layout(windows, socket, config, &self.output),
            Mode::Scroll => {
                if config.scroll.spread_windows_on_enter {
                    force_scroll_layout(windows, socket, config.scroll.column_width_percentage);
                }
            }
        }
    }
}
