use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use std::path::PathBuf;
use std::time::Duration;

use crate::service_state::Mode;

pub fn miri_socket_path() -> PathBuf {
    let runtime_dir =
        std::env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR not set. Are you running in a Wayland session?");
    PathBuf::from(runtime_dir).join("miri.sock")
}

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
            version: env!("CARGO_PKG_VERSION").into(),
            message,
        }
    }

    pub fn version_matches(&self) -> bool {
        self.version == env!("CARGO_PKG_VERSION")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IPCResponseContainer {
    pub version: String,
    pub response: MiriResponse,
}

impl IPCResponseContainer {
    pub fn new(response: MiriResponse) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            response,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MiriResponse {
    Ok,
    FocusedWorkspaceMode(Mode),
    Error(String),
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
    Override {
        #[command(subcommand)]
        override_action: MiriOverride,
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
}

#[derive(Debug, Clone, Subcommand, Serialize, Deserialize)]
pub enum MiriOverride {
    MoveColumnLeft,
    MoveColumnRight,
    MoveColumnToFirst,
    MoveColumnToLast,
    MoveColumnToMonitorUp,
    MoveColumnToMonitorDown,
    MoveColumnToMonitorLeft,
    MoveColumnToMonitorRight,
    MoveColumnToWorkspaceUp,
    MoveColumnToWorkspaceDown,
    MoveColumnToWorkspace { index: u8 },
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
pub enum MiriGet {
    FocusedWorkspaceMode,
}

#[derive(Debug)]
pub enum MiriServiceError {
    ConnectionFailed(String),
    SerializationFailed,
    SendFailed(String),
    ReceiveFailed(String),
    NoResponse,
    DeserializationFailed(String),
    Service(String),
}

impl std::fmt::Display for MiriServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiriServiceError::ConnectionFailed(_) => write!(
                f,
                "The miri service isn't running. Run `miri service start` or setup the systemd user service"
            ),
            MiriServiceError::NoResponse => write!(f, "The miri service did not respond"),
            MiriServiceError::Service(e) => write!(f, "The miri service could not handle the command: {}", e),
            MiriServiceError::SerializationFailed => write!(f, "Could not serialize the command"),
            MiriServiceError::SendFailed(e) => write!(f, "Could not send the command to the miri service: {}", e),
            MiriServiceError::ReceiveFailed(e) => write!(f, "Could not read the miri service's response: {}", e),
            MiriServiceError::DeserializationFailed(e) => {
                write!(f, "Could not parse the miri service's response: {}", e)
            }
        }
    }
}

pub fn send_command_to_miri_service(command: Command) -> Result<MiriResponse, MiriServiceError> {
    let mut stream =
        UnixStream::connect(miri_socket_path()).map_err(|e| MiriServiceError::ConnectionFailed(e.to_string()))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| MiriServiceError::ConnectionFailed(e.to_string()))?;

    let container = IPCMessageContainer::new(IPCMessage::CliExecute(command));
    let json = serde_json::to_string(&container).map_err(|_| MiriServiceError::SerializationFailed)?;

    let json_with_newline = format!("{}\n", json);
    stream
        .write_all(json_with_newline.as_bytes())
        .map_err(|e| MiriServiceError::SendFailed(e.to_string()))?;

    let mut line = String::new();
    let bytes_read = BufReader::new(&stream)
        .read_line(&mut line)
        .map_err(|e| MiriServiceError::ReceiveFailed(e.to_string()))?;

    if bytes_read == 0 {
        return Err(MiriServiceError::NoResponse);
    }

    let response_container: IPCResponseContainer =
        serde_json::from_str(&line).map_err(|e| MiriServiceError::DeserializationFailed(e.to_string()))?;

    match response_container.response {
        MiriResponse::Error(e) => Err(MiriServiceError::Service(e)),
        response => Ok(response),
    }
}
