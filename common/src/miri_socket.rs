use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::MIRI_SOCKET_PATH;

// TODO: this is an ai generated async socket reader. please redo this
pub struct MiriListener {
    listener: UnixListener,
}

pub struct MiriSocket {
    pub reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
}

impl MiriListener {
    pub async fn bind() -> Self {
        let _ = std::fs::remove_file(MIRI_SOCKET_PATH);
        let listener = UnixListener::bind(MIRI_SOCKET_PATH).expect("Failed to bind miri socket");
        Self { listener }
    }

    pub async fn accept(&self) -> MiriSocket {
        let (stream, _) = self.listener.accept().await.expect("Failed to accept connection");
        let (read_half, _) = tokio::io::split(stream);
        MiriSocket {
            reader: BufReader::new(read_half),
        }
    }
}

impl MiriSocket {
    pub async fn read(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.reader.read_line(&mut line).await {
            Ok(0) => None,
            Ok(_) => Some(line),
            Err(e) => {
                eprintln!("Error reading from miri socket: {}", e);
                None
            }
        }
    }
}
