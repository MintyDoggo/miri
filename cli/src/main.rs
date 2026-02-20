use clap::Parser;
use cli::miri_commands::*;
use common::{Args, Command, MiriAction, MiriGet};
use niri_ipc::{socket::Socket, Request};

trait CliRunner {
    fn run(&self, niri_ipc: Socket);
}

impl CliRunner for Command {
    fn run(&self, niri_ipc: Socket) {
        match self {
            Command::Action { action } => action.run(niri_ipc),
            Command::Get { get } => get.run(niri_ipc),
        }
    }
}

impl CliRunner for MiriAction {
    fn run(&self, mut _niri_ipc: Socket) {
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
    fn run(&self, mut niri_ipc: Socket) {
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

fn main() {
    let niri_ipc_socket =
        Socket::connect().expect("Failed to connect to niri ipc. Make sure you're using this inside a niri session");
    let args = Args::parse();
    args.command.run(niri_ipc_socket);
}
