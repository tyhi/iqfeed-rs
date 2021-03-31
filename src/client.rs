use std::cmp::min;

use async_channel::Sender;
use memmem::{Searcher, TwoWaySearcher};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::errors::ClientError;

pub struct IQFeed {
    stream: TcpStream,
    ice_breaker: TwoWaySearcher<'static>,
    tx: Sender<Vec<u8>>,
    buffer: Vec<u8>,
}

impl IQFeed {
    /// Created a new IQFeed Client connection
    ///
    /// # Errors
    pub async fn new(tx: Sender<Vec<u8>>) -> Result<Self, ClientError> {
        let mut stream = TcpStream::connect("").await?;
        stream.write_all(b"S,SET PROTOCOL,6.2\n").await?;
        Ok(Self {
            stream,
            ice_breaker: TwoWaySearcher::new(b"\n"),
            tx,
            buffer: Vec::new(),
        })
    }

    /// Sends a request to watch a symbol
    ///
    /// # Errors
    /// This will only error if there's an issue with the `TCPStream`. Any
    /// errors with watching the symbol will occur when `process` is called.
    pub async fn watch_trades(mut self, symbol: &str) -> Result<(), ClientError> {
        let command = format!("w{}\n", symbol.to_uppercase());
        Ok(self.stream.write_all(command.as_bytes()).await?)
    }

    /// Starts processing of the `TCPStream`. This should be sent to a tokio
    /// task.
    ///
    /// # Errors
    /// This will return an error if the Sender channel is closed.
    pub async fn process(mut self) -> Result<(), ClientError> {
        let mut buf = vec![0; 2048];
        let mut scan_read = 0;

        loop {
            let r = self.stream.read(&mut buf).await?;
            self.buffer.extend_from_slice(&buf[0..r]);

            loop {
                if let Some(e) = self.ice_breaker.search_in(&self.buffer[scan_read..]) {
                    if e == 0 {
                        self.buffer.drain(0..1);
                        continue;
                    };

                    self.tx.send(self.buffer.drain(0..(scan_read + e)).collect()).await?;
                } else {
                    scan_read = min(self.buffer.len() - 1, 0);
                    break;
                }
            }
        }
    }
}