use std::io::{Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::Path;

use crate::libc::{PollResult, PollSet};
use crate::pty::{self, Message, get_terminal_size};

static SIGWINCH_PIPE: std::sync::OnceLock<RawFd> = std::sync::OnceLock::new();

extern "C" fn sigwinch_handler(_: libc::c_int) {
    if let Some(&fd) = SIGWINCH_PIPE.get() {
        unsafe { libc::write(fd, [1u8].as_ptr() as *const libc::c_void, 1) };
    }
}

const CTRL_C: u8 = 0x03;
const CTRL_D: u8 = 0x04;

pub enum DetachReason {
    UserDetach,
    ProcessExited,
    ConnectionLost,
}

pub enum ClientEvent<'a> {
    /// stdin has data ready to read.
    Input(&'a [u8]),
    /// PTY output received from the shepherd.
    Output(&'a [u8]),
    /// The shepherd disconnected (process exited).
    Disconnected,
}

pub struct PtyClient {
    stream: UnixStream,
    sigwinch_r: std::io::PipeReader,
}

impl PtyClient {
    pub fn connect(path: &Path) -> std::io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        let (rows, cols) = get_terminal_size();
        let mut client = PtyClient::try_from(stream)?;
        client.send(Message::Resize { rows, cols });
        client.stream.set_nonblocking(true)?;
        Ok(client)
    }

    pub fn event_loop(mut self, readonly: bool) -> DetachReason {
        let original_termios = crate::pty::enter_raw_mode();

        let mut buf = [0u8; 4096];
        let reason = loop {
            match self.next_event(&mut buf) {
                ClientEvent::Input(data) => {
                    if data.contains(&CTRL_D) || (readonly && data.contains(&CTRL_C)) {
                        break DetachReason::UserDetach;
                    }
                    if !readonly && self.write_all(data).is_err() {
                        break DetachReason::ConnectionLost;
                    }
                }
                ClientEvent::Output(data) => {
                    if std::io::stdout().write_all(data).is_err() {
                        break DetachReason::ConnectionLost;
                    }
                    let _ = std::io::stdout().flush();
                }
                ClientEvent::Disconnected => break DetachReason::ProcessExited,
            }
        };

        if let Some(ref original) = original_termios {
            pty::restore_terminal(original);
        }

        reason
    }

    pub fn send(&mut self, msg: Message) {
        let _ = self.stream.write_all(&msg.encode());
    }

    /// Poll for the next event.
    /// Handles resize signals internally. Returns when stdin has data,
    /// the stream has output, or the connection drops.
    fn next_event<'a>(&mut self, buf: &'a mut [u8]) -> ClientEvent<'a> {
        let stdin = std::io::stdin();

        loop {
            let mut poll_set = PollSet::default();
            let stdin_idx = poll_set.add(&stdin);
            let stream_idx = poll_set.add(&self.stream);
            let sigwinch_idx = poll_set.add(&self.sigwinch_r);

            match poll_set.wait(1000) {
                PollResult::Interrupted | PollResult::Timeout => continue,
                PollResult::Error => return ClientEvent::Disconnected,
                PollResult::Ready => {}
            }

            // Handle resize internally
            if poll_set.is_readable(sigwinch_idx) {
                let _ = self.sigwinch_r.read(&mut [0u8; 16]);
                let (rows, cols) = get_terminal_size();
                self.send(Message::Resize { rows, cols });
                continue;
            }

            // Check stream before stdin — ensures server-sent data
            // (like log replay) is received before stdin is processed
            if poll_set.is_readable(stream_idx) {
                return match self.stream.read(buf) {
                    Ok(0) => ClientEvent::Disconnected,
                    Ok(n) => ClientEvent::Output(&buf[..n]),
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(_) => ClientEvent::Disconnected,
                };
            }

            if poll_set.is_hungup(stream_idx) {
                return ClientEvent::Disconnected;
            }

            if poll_set.is_readable(stdin_idx) {
                return match stdin.lock().read(buf) {
                    Ok(n) if n > 0 => ClientEvent::Input(&buf[..n]),
                    _ => continue,
                };
            }
        }
    }
}

impl Read for PtyClient {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for PtyClient {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl AsRawFd for PtyClient {
    fn as_raw_fd(&self) -> RawFd {
        self.stream.as_raw_fd()
    }
}

impl TryFrom<UnixStream> for PtyClient {
    type Error = std::io::Error;

    fn try_from(stream: UnixStream) -> Result<Self, Self::Error> {
        let (sigwinch_r, sigwinch_w) = std::io::pipe().expect("Failed to create SIGWINCH pipe");
        SIGWINCH_PIPE.set(sigwinch_w.as_raw_fd()).ok();
        std::mem::forget(sigwinch_w);
        unsafe { libc::signal(libc::SIGWINCH, sigwinch_handler as usize) };
        Ok(Self { stream, sigwinch_r })
    }
}
