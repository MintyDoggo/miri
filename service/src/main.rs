use common::config::MiriConfig;
use common::miri_socket::MiriListener;
use common::niri_socket::NiriSocket;
use common::{Command, IPCMessage, IPCMessageContainer, MiriAction, MiriGet, Mode};

use niri_ipc::Event;
use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Request, socket::Socket};

use service::mode_logic::master::{
    force_workspace_windows_into_layout_mode, handle_master_window_close, handle_master_window_open,
};
use service::niri_ipc_utils::{get_focused_workspace_mode, get_windows_on_focused_workspace, window_is_new};
use service::service_state::ServiceState;

use tokio::sync::mpsc::Sender;

trait CliRunner {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState);
}

impl CliRunner for Command {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState) {
        match self {
            Command::Action { action } => action.run(action_socket, event_state, service_state),
            Command::Get { get } => get.run(action_socket, event_state, service_state),
        }
    }
}

impl CliRunner for MiriAction {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState) {
        match self {
            MiriAction::CycleFocusedWorkspaceMode => {
                println!("[ACTION]: CycleFocusedWorkspaceMode");

                let Some(new_mode) = service_state
                    .workspace_modes
                    .cycle_mode_on_focused_workspace(&event_state)
                else {
                    eprintln!("Could not get new mode when cycling focused workspace mode");
                    return;
                };
                let Some(workspace_windows) = get_windows_on_focused_workspace(event_state) else {
                    eprintln!("Could not get workspace windows");
                    return;
                };
                force_workspace_windows_into_layout_mode(
                    workspace_windows,
                    action_socket,
                    &service_state.config,
                    new_mode,
                )
            }
            MiriAction::SetFocusedWorkspaceMode { mode } => {
                println!("[ACTION]: SetFocusedWorkspaceMode to {:?}", mode);
                service_state
                    .workspace_modes
                    .set_mode_on_focused_workspace(event_state, *mode);

                let Some(workspace_windows) = get_windows_on_focused_workspace(event_state) else {
                    eprintln!("Could not get workspace windows");
                    return;
                };

                force_workspace_windows_into_layout_mode(workspace_windows, action_socket, &service_state.config, *mode)
            }
            MiriAction::Spawn => {
                println!("[ACTION]: Spawn");
            }
        }
    }
}

impl CliRunner for MiriGet {
    fn run(&self, _action_socket: &mut Socket, _event_state: &EventStreamState, _service_state: &mut ServiceState) {
        match self {
            MiriGet::FocusedWorkspaceMode => {
                println!("[GET]: FocusedWorkspaceMode");
            }
            MiriGet::OtherThing => {
                println!("[GET]: OtherThing");
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
    match event {
        niri_ipc::Event::WindowOpenedOrChanged { ref window } => {
            if window_is_new(&window.id, event_state) {
                println!("[EVENT]: window opened");

                let Some(current_mode) = get_focused_workspace_mode(&service_state.workspace_modes, event_state) else {
                    eprintln!("Could not get focused workspace mode");
                    event_state.apply(event);
                    return;
                };

                println!("current mode {}", current_mode.as_str());

                match current_mode {
                    Mode::Master => handle_master_window_open(service_state, window, event_state, action_socket),
                    Mode::Scroll => {
                        event_state.apply(event);
                        return;
                    }
                }
            } else {
                println!("[EVENT]: window changed");
            }
        }
        niri_ipc::Event::WindowClosed { id: _ } => {
            println!("[EVENT]: window closed");
            let Some(current_mode) = get_focused_workspace_mode(&service_state.workspace_modes, event_state) else {
                eprintln!("Could not get focused workspace mode");
                event_state.apply(event);
                return;
            };
            match current_mode {
                Mode::Master => handle_master_window_close(service_state, event_state, action_socket),
                Mode::Scroll => {
                    event_state.apply(event);
                    return;
                }
            }
        }
        niri_ipc::Event::WindowsChanged { windows: _ } => {
            println!("[EVENT]: windows changed");
        }
        _ => {}
    }

    event_state.apply(event);
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<MiriEvent>(64);
    let mut action_socket = Socket::connect().expect("Failed to connect to niri_ipc action socket");
    let mut event_state = EventStreamState::default();
    let config = MiriConfig::load();
    let mut service_state = ServiceState::new(config);
    println!("{:?}", service_state.config);

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
