use std::io::{self, Write};
use tokio::sync::mpsc::UnboundedSender;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

struct ChannelWriter {
    tx: UnboundedSender<String>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf).to_string();
        let _ = self.tx.send(s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Initialize logging. If a channel is provided, log output is forwarded
/// to the channel instead of standard output.
pub fn init_logging(forward: Option<UnboundedSender<String>>) {
    if let Some(tx) = forward {
        let layer = fmt::layer().with_writer(move || ChannelWriter { tx: tx.clone() });
        tracing_subscriber::registry().with(layer).init();
    } else {
        tracing_subscriber::fmt::init();
    }
}

