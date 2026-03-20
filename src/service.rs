use niri_ipc::Event;
use niri_ipc::state::{EventStreamState, EventStreamStatePart};
use niri_ipc::{Request, socket::Socket};

use tokio::sync::mpsc::Sender;

use crate::config::MiriConfig;
use crate::ipc::{Command, IPCMessage, IPCMessageContainer, MiriAction, MiriGet, Mode};
use crate::layout::handler::{
    force_workspace_windows_into_layout_mode, handle_workspace_gain_window, handle_workspace_lose_window,
};
use crate::miri_socket::MiriListener;
use crate::niri_ipc_utils::{get_windows_on_focused_workspace, window_is_new};
use crate::niri_socket::NiriSocket;
use crate::service_state::{ServiceState, copy_event_state_to_layout};
trait CliRunner {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState);
}

impl CliRunner for Command {
    fn run(&self, action_socket: &mut Socket, event_state: &EventStreamState, service_state: &mut ServiceState) {
        match self {
            Command::Service { service_command: _ } => {}
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
                let focused_workspace = service_state
                    .current_layout
                    .get_focused_workspace_mut()
                    .expect("Could not get current focused workspace");
                focused_workspace.mode.cycle();
                let Some(workspace_windows) = get_windows_on_focused_workspace(event_state) else {
                    eprintln!("Could not get workspace windows");
                    return;
                };
                force_workspace_windows_into_layout_mode(
                    workspace_windows,
                    action_socket,
                    &service_state.config,
                    focused_workspace.mode,
                )
            }
            MiriAction::SetFocusedWorkspaceMode { mode } => {
                println!("[ACTION]: SetFocusedWorkspaceMode to {:?}", mode);
                service_state.current_layout.set_focused_workspace_mode(*mode);

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
    std::mem::swap(&mut service_state.previous_layout, &mut service_state.current_layout);
    // TODO: find a way to not have to clone the event
    event_state.apply(event.clone());
    // FIXME: dont make this pass by reference, just have it move the value out
    copy_event_state_to_layout(
        event_state,
        &service_state.previous_layout,
        &mut service_state.current_layout,
    );

    match event {
        niri_ipc::Event::WindowOpenedOrChanged { ref window } => {
            let current_workspace = service_state
                .current_layout
                .get_focused_workspace()
                .expect("Could not get current focused workspace");

            if window_is_new(
                &window.id,
                &service_state.previous_layout,
                &service_state.current_layout,
            ) {
                println!("[EVENT]: window opened");
                handle_workspace_gain_window(
                    current_workspace,
                    window,
                    &service_state.config,
                    action_socket,
                    service_state
                        .previous_layout
                        .get_focused_workspace()
                        .expect("Could not get previous focused workspace")
                        .get_focused_window(),
                );
            } else {
                println!("[EVENT]: window changed");
                let previous_focused_workspace = service_state
                    .previous_layout
                    .get_focused_workspace()
                    .expect("Could not get previous focused workspace");

                let window_moved_into_workspace = previous_focused_workspace.id
                    != service_state
                        .current_layout
                        .get_focused_workspace()
                        .expect("Could not get current focused workspace")
                        .id;

                if window_moved_into_workspace {
                    println!("[EVENT]: window moved to new workspace");
                    let previous_focused_workspace_current_state = service_state.current_layout.workspaces
                        .values()
                        .find(|workspace| workspace.id == previous_focused_workspace.id)
                        .expect("Could not get previous_focused_workspace_current_state. Somehow, a workspace was destroyed when a window moved to another workspace");

                    handle_workspace_lose_window(
                        previous_focused_workspace_current_state,
                        &service_state.config,
                        action_socket,
                    );
                    handle_workspace_gain_window(current_workspace, window, &service_state.config, action_socket, None)
                }
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
                Mode::Master => handle_workspace_lose_window(current_workspace, &service_state.config, action_socket),
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
    println!("MADE IT");
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
