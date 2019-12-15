use anyhow::Context;
use async_std::{fs::File, path::Path, prelude::*};
use srmw::*;
use std::{collections::HashMap, io::SeekFrom, time::Instant};

pub trait Progress {
    fn message(&mut self, message: &str);
    fn finish(&mut self);
    fn set(&mut self, value: u64);
}

#[derive(new)]
pub struct Task<P> {
    image: File,
    #[new(default)]
    pub writer: MultiWriter<File>,
    #[new(default)]
    pub state: HashMap<usize, (Box<Path>, P)>,
    check: bool,
}

impl<P: Progress> Task<P> {
    /// Performs the asynchronous USB device flashing.
    pub async fn process(mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        self.copy(buf).await.context("failed to copy ISO")?;
        self.flush().await.context("failed to flush devices")?;

        if self.check {
            self.seek().await.context("failed to seek devices to start")?;
            self.validate(buf).await.context("validation error")?;
        }

        for (_, pb) in self.state.values_mut() {
            pb.finish();
        }

        Ok(())
    }

    pub fn subscribe(&mut self, file: File, path: Box<Path>, progress: P) -> &mut Self {
        let entity = self.writer.insert(file);
        self.state.insert(entity, (path, progress));
        self
    }

    async fn copy(&mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        let mut stream = self.writer.copy(&mut self.image, buf);
        let mut total = 0;
        let mut last = Instant::now();
        while let Some(event) = stream.next().await {
            match event {
                CopyEvent::Progress(written) => {
                    total += written as u64;
                    let now = Instant::now();
                    if now.duration_since(last).as_millis() > 125 {
                        last = now;
                        for (_, pb) in self.state.values_mut() {
                            pb.set(total);
                        }
                    }
                }
                CopyEvent::Failure(entity, why) => {
                    let (path, mut pb) = self.state.remove(&entity).expect("missing entity");
                    pb.message(&format!("E {}: {}", path.display(), why));
                    pb.finish();
                }
                CopyEvent::SourceFailure(why) => {
                    for (path, pb) in self.state.values_mut() {
                        pb.message(&format!(
                            "E {}: error reading from source: {}",
                            path.display(),
                            why
                        ));
                        pb.finish();
                    }

                    return Err(why).context("error reading from source");
                }
                CopyEvent::NoWriters => return Err(anyhow!("no writers left")),
            }
        }

        Ok(())
    }

    async fn flush(&mut self) -> anyhow::Result<()> {
        for (path, pb) in self.state.values_mut() {
            pb.set(0);
            pb.message(&format!("F {}", path.display()));
        }

        let mut stream = self.writer.flush();
        while let Some((entity, why)) = stream.next().await {
            let (path, mut pb) = self.state.remove(&entity).expect("missing entity");
            pb.message(&format!("E {}: errored flushing to device: {}", path.display(), why));
            pb.finish();
        }

        Ok(())
    }

    async fn seek(&mut self) -> anyhow::Result<()> {
        for (path, pb) in self.state.values_mut() {
            pb.set(0);
            pb.message(&format!("S {}", path.display()));
        }

        let mut stream = self.writer.seek(SeekFrom::Start(0));
        while let Some((entity, why)) = stream.next().await {
            let (path, mut pb) = self.state.remove(&entity).expect("missing entity");
            pb.message(&format!("E {}: errored seeking to start: {}", path.display(), why));
            pb.finish();
        }

        Ok(())
    }

    async fn validate(&mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        for (path, pb) in self.state.values_mut() {
            pb.set(0);
            pb.message(&format!("V {}: ", path.display()));
        }

        let copy_bufs = &mut Vec::new();
        let mut total = 0;
        let mut stream = self.writer.validate(&mut self.image, buf, copy_bufs);

        while let Some(event) = stream.next().await {
            match event {
                ValidationEvent::Progress(written) => {
                    total += written as u64;
                    for (_, pb) in self.state.values_mut() {
                        pb.set(total);
                    }
                }
                ValidationEvent::Failure(entity, why) => {
                    let (path, mut pb) = self.state.remove(&entity).expect("missing entity");
                    pb.message(&format!("E {}: {}", path.display(), why));
                    pb.finish();
                }
                ValidationEvent::SourceFailure(why) => {
                    for (path, pb) in self.state.values_mut() {
                        pb.message(&format!(
                            "E {}: error reading from source: {}",
                            path.display(),
                            why
                        ));
                        pb.finish();
                    }

                    return Err(why).context("error reading from source");
                }
                ValidationEvent::NoWriters => return Err(anyhow!("no writers left")),
            }
        }

        Ok(())
    }
}
