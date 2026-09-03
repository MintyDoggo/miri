use niri_ipc::Event;
use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Request, socket::Socket};

use tokio::sync::mpsc::Sender;

use crate::config::MiriConfig;
use crate::ipc::{Command, IPCMessage, IPCMessageContainer, MiriAction, MiriGet};
use crate::layout::WorkspaceLayout;
use crate::miri_overrides::handle_override;
use crate::miri_socket::MiriListener;
use crate::niri_ipc_utils::{get_windows_on_focused_workspace, warn_if_version_mismatch};
use crate::niri_socket::NiriSocket;
use crate::service_state::{Mode, ServiceState, copy_event_state_to_layout};
trait CliRunner {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState);
}

impl CliRunner for Command {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState) {
        match self {
            Command::Service { service_command: _ } => {}
            Command::Action { action } => action.run(action_socket, event_state, service_state),
            Command::Get { get } => get.run(action_socket, event_state, service_state),
            Command::Override { override_action } => {
                handle_override(override_action.clone(), action_socket, service_state)
            }
        }
    }
}

impl CliRunner for MiriAction {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState) {
        // FIXME: i dont like the expect here
        let focused_workspace = service_state
            .current_layout
            .get_focused_workspace_mut()
            .expect("Could not get current focused workspace");
        let Some(workspace_windows) = get_windows_on_focused_workspace(event_state) else {
            eprintln!("Could not get workspace windows");
            return;
        };

        match self {
            MiriAction::CycleFocusedWorkspaceMode => {
                println!("[ACTION]: CycleFocusedWorkspaceMode");
                focused_workspace.mode.cycle();
            }
            MiriAction::SetFocusedWorkspaceMode { mode } => {
                println!("[ACTION]: SetFocusedWorkspaceMode to {:?}", mode);
                focused_workspace.mode = *mode;
            }
        }
        focused_workspace.force_mode(workspace_windows, action_socket, &service_state.config);
    }
}

impl CliRunner for MiriGet {
    fn run(&self, _action_socket: &mut Socket, _event_state: &EventStreamState, _service_state: &mut ServiceState) {
        match self {
            MiriGet::FocusedWorkspaceMode => {
                println!("[GET]: FocusedWorkspaceMode");
            }
        }
    }
}

enum MiriEvent {
    CliCommand(Command),
    NiriEvent(niri_ipc::Event),
    // i can easily add other event listeners here such as mouse, keyboard, etc. these would be part of THIS process
}

async fn run_cli_listener(tx: Sender<MiriEvent>) {
    let listener = MiriListener::bind().await;

    loop {
        let mut socket = listener.accept().await;
        while let Some(line) = socket.read().await {
            match serde_json::from_str::<IPCMessageContainer>(&line) {
                Ok(container) => {
                    let IPCMessage::CliExecute(command) = container.message;
                    if let Err(e) = tx.send(MiriEvent::CliCommand(command)).await {
                        eprintln!("Failed to send command to main loop: {}", e);
                    }
                }
                Err(e) => eprintln!("Failed to parse message '{}': {}", line.trim(), e),
            }
        }
    }
}

async fn run_niri_event_listener(tx: Sender<MiriEvent>) {
    let mut socket = NiriSocket::connect().await;
    socket.send(&Request::EventStream).await;

    loop {
        let line = socket.read().await;
        if let Ok(event) = serde_json::from_str::<niri_ipc::Event>(&line) {
            tx.send(MiriEvent::NiriEvent(event)).await.unwrap();
        }
    }
}

fn handle_niri_event(
    event: Event,
    event_state: &mut EventStreamState,
    service_state: &mut ServiceState,
    action_socket: &mut Socket,
) {
    std::mem::swap(&mut service_state.previous_layout, &mut service_state.current_layout);
    // TODO: find a way to not have to clone the event
    event_state.apply(event.clone());
    copy_event_state_to_layout(
        event_state,
        &service_state.previous_layout,
        &mut service_state.current_layout,
    );

    // make workspaces that already have windows default to scroll mode
    // this prevents scenarios where if miri.service crashes/restarts, the state doesnt remain broken
    // FIXME: we will obviously want this to never happen but its a good fix for now
    if !service_state.first_niri_event_received {
        // although this isn't the first event, its the first event where windows get assigned to workspaces (which is what we care about)
        if let niri_ipc::Event::WindowsChanged { .. } = event {
            service_state.first_niri_event_received = true;
            service_state.initialize_workspace_modes();
        }
    }

    match event {
        niri_ipc::Event::WindowOpenedOrChanged { ref window } => {
            if window.workspace_id.is_none() {
                // TODO: figure out how to handle this. this means its a window we are dragging (and possibly other cases)
                return;
            }

            let current_workspace = service_state
                .current_layout
                .get_focused_workspace()
                .expect("Could not get current focused workspace");

            let previous_workspace = service_state
                .previous_layout
                .get_focused_workspace()
                .expect("Could not get previous focused workspace");

            if ServiceState::window_is_new(previous_workspace, current_workspace, &window.id) {
                println!("[EVENT]: window opened");
                current_workspace.gain_window(
                    window,
                    &service_state.config,
                    action_socket,
                    previous_workspace.get_focused_window(),
                );
            } else {
                println!("[EVENT]: window changed");

                let window_moved_into_workspace = previous_workspace.id != current_workspace.id;

                if window_moved_into_workspace {
                    println!("[EVENT]: window moved to new workspace");
                    // assuming the workspace has changed, get the state of the previous focused workspace, but on the current state (this is hard to think about but it makes sense)
                    let previous_focused_workspace_current_state = service_state.current_layout.workspaces
                        .values()
                        .find(|workspace| workspace.id == previous_workspace.id)
                        .expect("Could not get previous_focused_workspace_current_state. Somehow, a workspace was destroyed when a window moved to another workspace");

                    previous_focused_workspace_current_state.lose_window(&service_state.config, action_socket);
                    current_workspace.gain_window(window, &service_state.config, action_socket, None);
                    return;
                }

                // handle window switching from tiling->floating and floating->tiling
                if let Some(previous_window) = previous_workspace
                    .windows
                    .iter()
                    .find(|previous_window| previous_window.id == window.id)
                {
                    match (previous_window.is_floating, window.is_floating) {
                        (true, false) => current_workspace.gain_window(
                            window,
                            &service_state.config,
                            action_socket,
                            previous_workspace.get_focused_window(),
                        ),
                        (false, true) => current_workspace.lose_window(&service_state.config, action_socket),
                        _ => {}
                    }
                };
            }
        }
        niri_ipc::Event::WindowClosed { id: _ } => {
            println!("[EVENT]: window closed");
            let current_workspace = service_state
                .current_layout
                .get_focused_workspace()
                .expect("Could not get current focused workspace");
            let current_mode = current_workspace.mode;
            match current_mode {
                Mode::Master => current_workspace.lose_window(&service_state.config, action_socket),
                Mode::Scroll => {
                    return;
                }
            }
        }
        niri_ipc::Event::WindowsChanged { windows: _ } => {
            println!("[EVENT]: windows changed");
        }
        _ => {}
    }
}

pub async fn main_service() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<MiriEvent>(64);
    let mut action_socket = Socket::connect().expect("Failed to connect to niri_ipc action socket");
    let mut event_state = EventStreamState::default();
    let config = MiriConfig::load();
    let mut service_state = ServiceState::new(config);

    warn_if_version_mismatch(&mut action_socket);

    tokio::spawn(run_cli_listener(tx.clone()));
    tokio::spawn(run_niri_event_listener(tx.clone()));

    while let Some(event) = rx.recv().await {
        match event {
            MiriEvent::CliCommand(command) => {
                command.run(&mut action_socket, &event_state, &mut service_state);
            }
            MiriEvent::NiriEvent(event) => {
                handle_niri_event(event, &mut event_state, &mut service_state, &mut action_socket)
            }
        }
    }
}
