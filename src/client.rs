use std::cmp::min;

use async_channel::Sender;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{errors::Error, models::Ops};

pub struct IQFeed {
    stream: TcpStream,
    tx: Sender<Ops>,
    buffer: Vec<u8>,
}

impl IQFeed {
    /// Created a new `IQFeed` Client connection, and sets the protocol to 6.2.
    ///
    /// # Errors
    ///
    /// # Examples
    /// ```no_run
    /// use async_channel::unbounded;
    /// use iqfeed_rs::client::IQFeed;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (rx, tx) = unbounded();
    ///     let client = IQFeed::new(rx, "localhost:5009").await.unwrap();
    /// }
    /// ```
    pub async fn new(tx: Sender<Ops>, addr: &str) -> Result<Self, Error> {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(b"S,SET PROTOCOL,6.2\n").await?;
        Ok(Self {
            stream,
            tx,
            buffer: Vec::new(),
        })
    }

    /// Sends a request to watch a symbol
    ///
    /// # Errors
    /// This will only error if there's an issue with the `TCPStream`. Any
    /// errors with watching the symbol will occur when `process` is called.
    ///
    /// # Examples
    /// ```no_run
    /// use async_channel::unbounded;
    /// use iqfeed_rs::client::IQFeed;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (rx, tx) = unbounded();
    ///     let mut client = IQFeed::new(rx, "localhost:5009").await.unwrap();
    ///     client.watch_trades("PLTR").await.unwrap();
    /// }
    /// ```
    pub async fn watch_trades(&mut self, symbol: &str) -> Result<(), Error> {
        let command = format!("w{}\n", symbol.to_uppercase());
        Ok(self.stream.write_all(command.as_bytes()).await?)
    }

    /// Starts processing of the `TCPStream`. This should be sent to a tokio
    /// task.
    ///
    /// # Errors
    /// This will return an error if the Sender channel is closed.
    ///
    /// # Examples
    /// ```no_run
    /// use async_channel::unbounded;
    /// use iqfeed_rs::client::IQFeed;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (rx, tx) = unbounded();
    ///     let mut client = IQFeed::new(rx, "localhost:5009").await.unwrap();
    ///     client.watch_trades("PLTR").await.unwrap();
    ///
    ///     // Spawning a tokio task to run the process is the best way as
    ///     // ideally you would have multiple connections to the IQFeed client
    ///     tokio::spawn(async move { client.process() });
    /// }
    /// ```
    pub async fn process(mut self) -> Result<(), Error> {
        let mut buf = vec![0; 2048];
        let mut scan_read = 0;

        loop {
            let r = self.stream.read(&mut buf).await?;
            self.buffer.extend_from_slice(&buf[0..r]);

            loop {
                if let Some(e) = memchr::memchr(b'\n', &self.buffer[scan_read..]) {
                    if e == 0 {
                        self.buffer.drain(0..1);
                        break;
                    };

                    self.tx
                        .send(Ops::parse(&self.buffer.drain(0..(scan_read + e)).collect::<Vec<_>>())?)
                        .await?;
                } else {
                    scan_read = min(self.buffer.len() - 1, 0);
                    break;
                }
            }
        }
    }
}
