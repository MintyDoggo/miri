use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

pub mod config;
pub mod miri_socket;
pub mod niri_socket;

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
    Action {
        #[command(subcommand)]
        action: MiriAction,
    },
    Get {
        #[command(subcommand)]
        get: MiriGet,
    },
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
pub enum MiriAction {
    CycleFocusedWorkspaceMode,
    Spawn,
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
pub enum MiriGet {
    FocusedWorkspaceMode,
    OtherThing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Scroll,
    Master,
}

impl Mode {
    pub fn cycle(self) -> Mode {
        match self {
            Mode::Scroll => Mode::Master,
            Mode::Master => Mode::Scroll,
        }
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
