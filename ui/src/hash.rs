// SPDX-License-Identifier: MIT

use digest::Digest;
use hex_view::HexView;
use std::io;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};

pub(crate) async fn hasher<H: Digest>(image: &Path) -> io::Result<String> {
    let file = File::open(image).await?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut buffer = [0u8; 64 * 1024];
    let mut hasher = H::new();

    loop {
        let read = reader.read(&mut buffer).await?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", HexView::from(hasher.finalize().as_slice())))
}

#[derive(Debug, Clone, Copy)]
pub enum HashResult {
    Checking,
    Match,
    Mismatch,
    Error,
}
