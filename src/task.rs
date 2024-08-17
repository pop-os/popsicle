use anyhow::Context;
use async_std::{fs::File, prelude::*};
use srmw::*;
use std::{collections::HashMap, io::SeekFrom, time::Instant};

pub trait Progress {
    type Device;
    fn message(&mut self, device: &Self::Device, kind: &str, message: &str);
    fn finish(&mut self);
    fn set(&mut self, value: u64);
}

#[derive(new)]
pub struct Task<P: Progress> {
    image: File,

    #[new(default)]
    pub writer: MultiWriter<File>,

    #[new(default)]
    pub state: HashMap<usize, (P::Device, P)>,

    #[new(value = "125")]
    pub millis_between: u64,

    check: bool,
}

impl<P: Progress> Task<P> {
    /// Performs the asynchronous USB device flashing.
    pub async fn process(mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        self.copy(buf).await.context("failed to copy ISO")?;

        if self.check {
            self.seek().await.context("failed to seek devices to start")?;
            self.validate(buf).await.context("validation error")?;
        }

        for (_, pb) in self.state.values_mut() {
            pb.finish();
        }

        Ok(())
    }

    pub fn subscribe(&mut self, file: File, device: P::Device, progress: P) -> &mut Self {
        let entity = self.writer.insert(file);
        self.state.insert(entity, (device, progress));
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
                    if now.duration_since(last).as_millis() > self.millis_between as u128 {
                        last = now;
                        for (_, pb) in self.state.values_mut() {
                            pb.set(total);
                        }
                    }
                }
                CopyEvent::Failure(entity, why) => {
                    let (device, mut pb) = self.state.remove(&entity).expect("missing entity");
                    pb.message(&device, "E", &format!("{}", why));
                    pb.finish();
                }
                CopyEvent::SourceFailure(why) => {
                    for (device, pb) in self.state.values_mut() {
                        pb.message(device, "E", &format!("{}", why));
                        pb.finish();
                    }

                    return Err(why).context("error reading from source");
                }
                CopyEvent::NoWriters => return Err(anyhow!("no writers left")),
            }
        }

        Ok(())
    }

    async fn seek(&mut self) -> anyhow::Result<()> {
        for (path, pb) in self.state.values_mut() {
            pb.set(0);
            pb.message(path, "S", "");
        }

        self.image.seek(SeekFrom::Start(0)).await?;

        let mut stream = self.writer.seek(SeekFrom::Start(0));
        while let Some((entity, why)) = stream.next().await {
            let (path, mut pb) = self.state.remove(&entity).expect("missing entity");
            pb.message(&path, "E", &format!("errored seeking to start: {}", why));
            pb.finish();
        }

        Ok(())
    }

    async fn validate(&mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        for (path, pb) in self.state.values_mut() {
            pb.set(0);
            pb.message(path, "V", "");
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
                    pb.message(&path, "E", &format!("{}", why));
                    pb.finish();
                }
                ValidationEvent::SourceFailure(why) => {
                    for (path, pb) in self.state.values_mut() {
                        pb.message(path, "E", &format!("error reading from source: {}", why));
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
