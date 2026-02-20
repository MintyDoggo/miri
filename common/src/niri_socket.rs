use niri_ipc::Request;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

// TODO: this is an ai generated niri async socket reader. please redo this
pub struct NiriSocket {
    pub reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    pub writer: tokio::io::WriteHalf<UnixStream>,
}

impl NiriSocket {
    pub async fn connect() -> Self {
        let path = std::env::var("NIRI_SOCKET").expect("NIRI_SOCKET not set");
        let stream = UnixStream::connect(path)
            .await
            .expect("failed to connect to niri socket");
        let (read_half, write_half) = tokio::io::split(stream);
        Self {
            reader: BufReader::new(read_half),
            writer: write_half,
        }
    }

    pub async fn send(&mut self, request: &Request) -> String {
        let mut payload = serde_json::to_string(request).expect("Could not convert json to string");
        payload.push('\n');
        self.writer
            .write_all(payload.as_bytes())
            .await
            .expect("Could not write to socket");

        let mut line = String::new();
        self.reader.read_line(&mut line).await.unwrap();
        line
    }

    pub async fn read(&mut self) -> String {
        let mut line = String::new();
        self.reader.read_line(&mut line).await.unwrap();
        line
    }
}
