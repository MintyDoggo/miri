use clap::Parser;
use miri::{
    ipc::{Args, Command, MiriAction, MiriGet, MiriOverride, MiriServiceCommand, send_command_to_miri_service},
    miri_overrides,
    service::main_service,
};
use niri_ipc::socket::Socket;

trait CliRunner {
    async fn run(&self, niri_ipc: Socket);
}

impl CliRunner for MiriAction {
    async fn run(&self, mut _niri_ipc: Socket) {
        match self {
            MiriAction::CycleFocusedWorkspaceMode => {
                if let Err(e) = send_command_to_miri_service(Command::Action {
                    action: MiriAction::CycleFocusedWorkspaceMode,
                }) {
                    eprintln!("Failed to send action to miri service: {:?}", e);
                }
            }
            MiriAction::SetFocusedWorkspaceMode { mode: _ } => {
                if let Err(e) = send_command_to_miri_service(Command::Action { action: self.clone() }) {
                    eprintln!("Failed to send action to miri service: {:?}", e);
                }
            }
            MiriAction::WindowToggleScratchPad { window_id: _ } => {
                if let Err(e) = send_command_to_miri_service(Command::Action { action: self.clone() }) {
                    eprintln!("Failed to send action to miri service: {:?}", e);
                }
            }
        }
    }
}

impl CliRunner for MiriGet {
    async fn run(&self, mut _niri_ipc: Socket) {
        match self {
            MiriGet::FocusedWorkspaceMode => {
                send_command_to_miri_service(Command::Get {
                    get: MiriGet::FocusedWorkspaceMode,
                })
                .expect("Get commands require the miri service to be running. Run `miri service start` or setup the systemd user service");
            }
        }
    }
}

impl CliRunner for MiriOverride {
    async fn run(&self, mut niri_ipc: Socket) {
        match send_command_to_miri_service(Command::Override {
            override_action: self.clone(),
        }) {
            Ok(()) => {}
            Err(_) => {
                miri_overrides::scroll_passthrough(self.clone(), &mut niri_ipc);
            }
        }
    }
}

impl CliRunner for Command {
    async fn run(&self, niri_ipc: Socket) {
        match self {
            Command::Service { service_command } => match service_command {
                MiriServiceCommand::Start => main_service().await,
            },
            Command::Action { action } => action.run(niri_ipc).await,
            Command::Get { get } => get.run(niri_ipc).await,
            Command::Override { override_action } => override_action.run(niri_ipc).await,
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let niri_ipc_socket =
        Socket::connect().expect("Failed to connect to niri ipc. Make sure you're using this inside a niri session");
    args.command.run(niri_ipc_socket).await;
}
