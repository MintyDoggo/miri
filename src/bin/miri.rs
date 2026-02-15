use clap::{Parser, Subcommand};
use niri_ipc::{Action, Request, socket::Socket};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Action {
        #[command(subcommand)]
        action: MiriAction,
    },
    Get {
        #[command(subcommand)]
        get: MiriGet,
    },
}

impl Commands {
    fn run(&self, niri_ipc: Socket) {
        match self {
            Commands::Action { action } => action.run(niri_ipc),
            Commands::Get { get } => get.run(niri_ipc),
        }
    }
}

#[derive(Subcommand)]
enum MiriAction {
    CycleFocusedWorkspaceMode,
    Spawn,
}

impl MiriAction {
    fn run(&self, mut niri_ipc: Socket) {
        match self {
            MiriAction::CycleFocusedWorkspaceMode => {}
            MiriAction::Spawn => {}
        }
    }
}

#[derive(Subcommand)]
enum MiriGet {
    FocusedWorkspaceMode,
    OtherThing,
}

impl MiriGet {
    fn run(&self, mut niri_ipc: Socket) {
        match self {
            MiriGet::FocusedWorkspaceMode => {
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
    let niri_ipc_socket = Socket::connect().expect(
        "Failed to connect to niri ipc. Make sure you're using this inside a niri session.",
    );
    let args = Args::parse();

    args.command.run(niri_ipc_socket);
}
