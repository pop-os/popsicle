use futures_codec::{BytesMut, Decoder};
use memchr::memchr;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};
use thiserror::Error;

/// Errors that may occur when decoding the IPC stream.
#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to decode popsicle message: {{\n  {}\n}}", input)]
    Decode { input: Box<str>, source: ron::de::SpannedError },
    #[error("reading from popsicle stream failed")]
    Read(#[from] io::Error),
}

/// Popsicle's IPC protocol
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub enum Message {
    Device(PathBuf),
    Finished(PathBuf),
    Message(PathBuf, String),
    Set(PathBuf, u64),
    Size(u64),
}

/// A decoder for creating a stream of messages from a reader
///
/// ```ignore
/// use futures_code::FramedRead;
///
/// FramedRead::new(pipe_reader, PopsicleDecoder::default())
/// ```
#[derive(Default)]
pub struct PopsicleDecoder;

impl Decoder for PopsicleDecoder {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match memchr(b'\n', src) {
            Some(pos) => {
                let buf = src.split_to(pos + 1);
                match ron::de::from_bytes::<Self::Item>(&buf) {
                    Ok(value) => Ok(Some(value)),
                    Err(source) => Err(Error::Decode {
                        input: String::from_utf8_lossy(&buf).into_owned().into(),
                        source,
                    }),
                }
            }
            None => Ok(None),
        }
    }
}
