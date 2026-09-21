use clap::Parser;
use miri::{
    ipc::{
        Args, Command, MiriAction, MiriGet, MiriOverride, MiriResponse, MiriServiceCommand, MiriServiceError,
        send_command_to_miri_service,
    },
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
                    eprintln!("{}", e);
                }
            }
            MiriAction::SetFocusedWorkspaceMode { mode: _ } => {
                if let Err(e) = send_command_to_miri_service(Command::Action { action: self.clone() }) {
                    eprintln!("{}", e);
                }
            }
        }
    }
}

impl CliRunner for MiriGet {
    async fn run(&self, mut _niri_ipc: Socket) {
        match self {
            MiriGet::FocusedWorkspaceMode => {
                match send_command_to_miri_service(Command::Get {
                    get: MiriGet::FocusedWorkspaceMode,
                }) {
                    Ok(MiriResponse::FocusedWorkspaceMode(mode)) => println!("{}", mode.as_str()),
                    Ok(response) => eprintln!("The miri service returned an unexpected response: {:?}", response),
                    Err(e) => eprintln!("{}", e),
                }
            }
        }
    }
}

impl CliRunner for MiriOverride {
    async fn run(&self, mut niri_ipc: Socket) {
        match send_command_to_miri_service(Command::Override {
            override_action: self.clone(),
        }) {
            Ok(_) => {}
            Err(MiriServiceError::ConnectionFailed(_)) => {
                miri_overrides::scroll_passthrough(self.clone(), &mut niri_ipc);
            }
            Err(e) => eprintln!("{}", e),
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
