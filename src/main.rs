use clap::Parser;
use miri::{
    ipc::{Args, Command, MiriAction, MiriGet, MiriServiceCommand, send_command_to_miri_service},
    service::main_service,
};
use niri_ipc::{Request, socket::Socket};

trait CliRunner {
    async fn run(&self, niri_ipc: Socket);
}

impl CliRunner for MiriAction {
    async fn run(&self, mut _niri_ipc: Socket) {
        match self {
            MiriAction::CycleFocusedWorkspaceMode => {
                send_command_to_miri_service(Command::Action {
                    action: MiriAction::CycleFocusedWorkspaceMode,
                });
            }
            MiriAction::SetFocusedWorkspaceMode { mode: _ } => {
                send_command_to_miri_service(Command::Action { action: self.clone() });
            }
            MiriAction::Spawn => {}
        }
    }
}

impl CliRunner for MiriGet {
    async fn run(&self, mut niri_ipc: Socket) {
        match self {
            MiriGet::FocusedWorkspaceMode => {
                send_command_to_miri_service(Command::Get {
                    get: MiriGet::FocusedWorkspaceMode,
                });
                match niri_ipc.send(Request::Workspaces) {
                    Ok(Ok(response)) => print!("{:?}", response),
                    Ok(Err(message)) => print!("{:?}", message),
                    Err(error) => print!("{:?}", error),
                };
            }
            MiriGet::OtherThing => {}
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
