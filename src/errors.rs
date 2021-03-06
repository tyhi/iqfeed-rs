use thiserror::Error;
use tokio::io;

use crate::models::Ops;

/// `ParsingError` is an error returned from anything having to do with parsing
/// data.
#[derive(Error, Debug)]
pub enum Error {
    #[error("error parsing number")]
    Number(#[from] lexical::Error),
    #[error("error parsing time")]
    Timestamp(#[from] time::error::Parse),
    #[error("error parsing float")]
    Float(#[from] fast_float::Error),
    #[error("error parsing number")]
    Tcp(#[from] io::Error),
    #[error("error sending msg over channel")]
    Channel(#[from] async_channel::SendError<Ops>),
}
