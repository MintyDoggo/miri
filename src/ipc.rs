use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::net::UnixStream;

pub const MIRI_SOCKET_PATH: &str = "/tmp/modal-niri.sock";

#[derive(Debug, Serialize, Deserialize)]
pub struct IPCMessageContainer {
    pub version: String,
    pub message: IPCMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IPCMessage {
    CliExecute(Command),
}

impl IPCMessageContainer {
    pub fn new(message: IPCMessage) -> Self {
        Self {
            version: "1.0".to_string(),
            message,
        }
    }
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
pub enum Command {
    Service {
        #[command(subcommand)]
        service_command: MiriServiceCommand,
    },
    Action {
        #[command(subcommand)]
        action: MiriAction,
    },
    Get {
        #[command(subcommand)]
        get: MiriGet,
    },
}

#[derive(Debug, Clone, Subcommand, Serialize, Deserialize)]
pub enum MiriServiceCommand {
    Start,
}

#[derive(Debug, Clone, Subcommand, Serialize, Deserialize)]
pub enum MiriAction {
    CycleFocusedWorkspaceMode,
    SetFocusedWorkspaceMode { mode: Mode },
    Spawn,
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
pub enum MiriGet {
    FocusedWorkspaceMode,
    OtherThing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum Mode {
    Scroll,
    Master,
}

impl Mode {
    pub fn cycle(&mut self) {
        *self = match *self {
            Mode::Scroll => Mode::Master,
            Mode::Master => Mode::Scroll,
        };
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Scroll => "scroll",
            Mode::Master => "master",
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Scroll
    }
}

// send a command to the daemon
pub fn send_command_to_miri_service(command: Command) {
    match UnixStream::connect(MIRI_SOCKET_PATH) {
        Ok(mut stream) => {
            let container = IPCMessageContainer::new(IPCMessage::CliExecute(command));
            let json = serde_json::to_string(&container).expect("Failed to serialize command");
            let json_with_newline = format!("{}\n", json);

            if let Err(e) = stream.write_all(json_with_newline.as_bytes()) {
                eprintln!("Failed to send command: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to miri service: {e}");
            std::process::exit(1);
        }
    }
}
