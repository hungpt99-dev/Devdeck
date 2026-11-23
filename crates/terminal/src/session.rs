//! PTY child lifecycle: bounded events, coalesced output, idempotent control.
//!
//! The PTY master is a blocking byte stream, so a dedicated reader thread
//! copies it into a bounded `tokio::sync::mpsc` channel. The async side drains
//! that channel in batches (coalescing) before handing bytes to the vte grid.

use std::io::{Read, Write};
use std::path::Path;
use std::thread::{self, JoinHandle};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::TerminalSettings;

/// Maximum queued PTY events. A stalled consumer applies backpressure to the
/// reader thread instead of accumulating unbounded output.
pub const EVENT_CHANNEL_CAPACITY: usize = 256;
const COALESCE_MAX_EVENTS: usize = 64;
const COALESCE_MAX_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("PTY setup failed: {0}")]
    Pty(String),
    #[error("shell spawn failed: {0}")]
    Spawn(String),
    #[error("terminal I/O failed: {0}")]
    Io(String),
    #[error("invalid terminal size")]
    InvalidSize,
}

#[derive(Debug, Default)]
pub struct Drain {
    pub output: Vec<u8>,
    pub exited: bool,
    pub error: Option<String>,
}

#[derive(Debug)]
enum SessionEvent {
    Output(Vec<u8>),
    Exited,
    Failed(String),
}

pub struct Session {
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child: Option<Box<dyn Child + Send>>,
    reader: Option<JoinHandle<()>>,
    events: Receiver<SessionEvent>,
    size: PtySize,
    closed: bool,
}

impl Session {
    /// Spawn the platform shell in `cwd`. `command`, when present, is run via
    /// the shell's `-c`/`/C` form. Callers must have already obtained an
    /// approval decision for the command.
    pub fn spawn(
        cwd: &Path,
        settings: &TerminalSettings,
        command: Option<&str>,
    ) -> Result<Self, SessionError> {
        let size = PtySize {
            rows: settings.rows,
            cols: settings.cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = native_pty_system()
            .openpty(size)
            .map_err(|e| SessionError::Pty(e.to_string()))?;
        let mut builder = shell_command(command);
        builder.cwd(cwd);
        // Interactive shells otherwise write a history file that can contain
        // the very secrets the redactor removes from output and audit.
        builder.env("HISTFILE", if cfg!(unix) { "/dev/null" } else { "NUL" });
        builder.env("HISTCONTROL", "ignoredups:ignorespace");
        builder.env("HISTFILESIZE", "0");
        builder.env("HISTSIZE", "0");
        builder.env("TERM", "xterm-256color");
        let child = pair
            .slave
            .spawn_command(builder)
            .map_err(|e| SessionError::Spawn(e.to_string()))?;
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| SessionError::Io(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| SessionError::Io(e.to_string()))?;
        let (sender, events) = mpsc::channel(settings.event_capacity.max(1));
        Ok(Self {
            master: Some(pair.master),
            writer: Some(writer),
            child: Some(child),
            reader: Some(spawn_reader(reader, sender)),
            events,
            size,
            closed: false,
        })
    }

    /// Test seam: drive the event/coalescing machinery without a real shell so
    /// the lifecycle tests run identically on Linux, macOS and Windows.
    #[cfg(test)]
    fn from_io(
        reader: Box<dyn Read + Send>,
        writer: Box<dyn Write + Send>,
        size: PtySize,
        capacity: usize,
    ) -> Self {
        let (sender, events) = mpsc::channel(capacity);
        Self {
            master: None,
            writer: Some(writer),
            child: None,
            reader: Some(spawn_reader(reader, sender)),
            events,
            size,
            closed: false,
        }
    }

    pub fn size(&self) -> PtySize {
        self.size
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Drain one batch of PTY events. Repeated calls after EOF return an empty
    /// `Drain` rather than blocking forever.
    pub async fn drain(&mut self) -> Drain {
        let mut result = Drain::default();
        let mut events = 0usize;
        while events < COALESCE_MAX_EVENTS && result.output.len() < COALESCE_MAX_BYTES {
            let event = if events == 0 {
                self.events.recv().await
            } else {
                // The reader runs on its own thread: an empty try_recv here does
                // not mean EOF, only that the next chunk has not been queued yet.
                // Wait briefly so a still-producing session is not truncated.
                match tokio::time::timeout(std::time::Duration::from_millis(20), self.events.recv())
                    .await
                {
                    Ok(event) => event,
                    Err(_) => break,
                }
            };
            match event {
                Some(SessionEvent::Output(bytes)) => result.output.extend_from_slice(&bytes),
                Some(SessionEvent::Exited) => {
                    result.exited = true;
                    break;
                }
                Some(SessionEvent::Failed(message)) => {
                    result.error = Some(message);
                    break;
                }
                None => {
                    result.exited = true;
                    break;
                }
            }
            events += 1;
        }
        result
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<(), SessionError> {
        if self.closed {
            return Err(SessionError::Io("session is closed".into()));
        }
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| SessionError::Io("no PTY writer".into()))?;
        writer
            .write_all(bytes)
            .map_err(|e| SessionError::Io(e.to_string()))
    }

    /// Resize is idempotent: same size or a closed session is a no-op.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        if cols == 0 || rows == 0 {
            return Err(SessionError::InvalidSize);
        }
        if self.closed || (self.size.cols == cols && self.size.rows == rows) {
            return Ok(());
        }
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        if let Some(master) = self.master.as_ref() {
            master
                .resize(size)
                .map_err(|e| SessionError::Io(e.to_string()))?;
        }
        self.size = size;
        Ok(())
    }

    /// Close is idempotent: writer drop sends EOF, the child is terminated and
    /// the reader is left to finish once the channel receiver goes away.
    pub fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        drop(self.writer.take());
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(reader) = self.reader.take() {
            for _ in 0..20 {
                if reader.is_finished() {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(5));
            }
            // Dropping the handle detaches the reader; it exits on the next
            // failed channel send once the Session (and its receiver) drops.
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

fn spawn_reader(mut reader: Box<dyn Read + Send>, sender: Sender<SessionEvent>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.blocking_send(SessionEvent::Exited);
                    break;
                }
                Ok(n) => {
                    if sender
                        .blocking_send(SessionEvent::Output(buffer[..n].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    let _ = sender.blocking_send(SessionEvent::Failed(error.to_string()));
                    break;
                }
            }
        }
    })
}

#[cfg(unix)]
fn shell_command(command: Option<&str>) -> CommandBuilder {
    match command {
        Some(command) => {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
            let mut builder = CommandBuilder::new(shell);
            builder.arg("-c");
            builder.arg(command);
            builder
        }
        None => CommandBuilder::new_default_prog(),
    }
}

#[cfg(windows)]
fn shell_command(command: Option<&str>) -> CommandBuilder {
    match command {
        Some(command) => {
            let mut builder = CommandBuilder::new("cmd.exe");
            builder.arg("/C");
            builder.arg(command);
            builder
        }
        None => CommandBuilder::new_default_prog(),
    }
}

#[cfg(not(any(unix, windows)))]
fn shell_command(_command: Option<&str>) -> CommandBuilder {
    CommandBuilder::new_default_prog()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    use super::*;

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("writer mutex")
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn size() -> PtySize {
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    #[tokio::test]
    async fn bounded_channel_coalesces_output() {
        let payload = vec![b'x'; 8 * 1024];
        let mut session = Session::from_io(
            Box::new(Cursor::new(payload.clone())),
            Box::new(SharedWriter(Arc::new(Mutex::new(Vec::new())))),
            size(),
            4,
        );
        let drain = session.drain().await;
        assert_eq!(drain.output, payload);
        assert!(drain.exited);
    }

    #[tokio::test]
    async fn write_reaches_the_master_and_resize_close_are_idempotent() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let mut session = Session::from_io(
            Box::new(Cursor::new(Vec::new())),
            Box::new(SharedWriter(Arc::clone(&buffer))),
            size(),
            4,
        );
        session.write(b"echo hi\n").unwrap();
        assert_eq!(*buffer.lock().unwrap(), b"echo hi\n");

        assert_eq!(session.size(), size());
        session.resize(80, 24).unwrap();
        assert_eq!(session.size(), size());
        session.resize(100, 30).unwrap();
        assert_eq!(session.size().cols, 100);
        assert!(matches!(
            session.resize(0, 30),
            Err(SessionError::InvalidSize)
        ));

        session.close();
        session.close();
        assert!(session.is_closed());
        session.resize(120, 40).unwrap();
        assert_eq!(session.size().cols, 100);
        assert!(session.write(b"no").is_err());
    }
}
