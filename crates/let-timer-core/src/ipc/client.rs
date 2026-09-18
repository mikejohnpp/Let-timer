use std::path::Path;

use crate::protocol::{Command, Response};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

pub struct IpcClient {
    read_half: BufReader<OwnedReadHalf>,
    write_half: OwnedWriteHalf,
}

impl IpcClient {
    pub async fn connect(path: &Path) -> Result<Self, std::io::Error> {
        let stream = UnixStream::connect(path).await?;
        let (read_half, write_half) = stream.into_split();

        Ok(Self {
            read_half: BufReader::new(read_half),
            write_half,
        })
    }

    pub async fn request(
        &mut self,
        command: &Command,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(command)?;

        self.write_half.write_all(json.as_bytes()).await?;
        self.write_half.write_all(b"\n").await?;

        let mut line = String::new();
        let n = self.read_half.read_line(&mut line).await?;
        if n == 0 {
            return Err("Server closed connection".into());
        }

        Ok(serde_json::from_str::<Response>(line.trim())?)
    }
}
