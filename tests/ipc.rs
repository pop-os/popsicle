use futures::{executor, io::AllowStdIo, prelude::*};
use futures_codec::FramedRead;
use popsicle::codec::*;
use std::io::Cursor;

const SAMPLE: &[u8] = include_bytes!("ipc.ron");

#[test]
fn ipc() {
    executor::block_on(async move {
        let expected = vec![
            Message::Size(2229190656),
            Message::Device("/dev/sdb".into()),
            Message::Device("/dev/sda".into()),
            Message::Set("/dev/sda".into(), 589824),
            Message::Set("/dev/sdb".into(), 589824),
            Message::Set("/dev/sdb".into(), 384434176),
            Message::Set("/dev/sda".into(), 1669005312),
            Message::Set("/dev/sdb".into(), 2228748288),
            Message::Set("/dev/sda".into(), 0),
            Message::Message("/dev/sda".into(), "S".into()),
            Message::Set("/dev/sdb".into(), 0),
            Message::Message("/dev/sdb".into(), "S".into()),
            Message::Set("/dev/sda".into(), 0),
            Message::Message("/dev/sda".into(), "V".into()),
            Message::Set("/dev/sdb".into(), 0),
            Message::Message("/dev/sdb".into(), "V".into()),
            Message::Finished("/dev/sda".into()),
            Message::Finished("/dev/sdb".into()),
        ];

        let input = AllowStdIo::new(Cursor::new(SAMPLE));

        let mut stream = FramedRead::new(input, PopsicleDecoder::default());

        let mut expected_iter = expected.iter();

        let mut matched = 0;
        while let Some(message) = stream.next().await {
            let message = message.unwrap();

            assert_eq!(message, *expected_iter.next().unwrap());
            matched += 1;
        }

        assert_eq!(matched, expected.len());
    });
}
