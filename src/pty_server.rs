use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use crate::libc::{PollResult, PollSet, has_child_exited};
use crate::pty::{Message, set_window_size};

pub enum ServerEvent<'a> {
    /// PTY master produced output.
    Output(&'a [u8]),
    /// Client sent input — write to PTY master.
    ClientInput(Vec<u8>),
    /// Client requested a resize — resize the PTY.
    ClientResize(u16, u16),
    /// PTY master closed or child process exited — shepherd should stop.
    Done,
}

pub struct PtyServer {
    listener: UnixListener,
    clients: Vec<UnixStream>,
    client_buf: [u8; 4096],
    vt: avt::Vt,
}

impl PtyServer {
    pub fn bind(path: &Path, rows: u16, cols: u16) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path)?;
        listener.set_nonblocking(true)?;
        Ok(PtyServer {
            listener,
            clients: Vec::new(),
            client_buf: [0u8; 4096],
            vt: avt::Vt::builder()
                .size(cols as usize, rows as usize)
                .scrollback_limit(10000)
                .build(),
        })
    }

    pub fn event_loop(mut self, master: &mut File, child_pid: i32) {
        let mut buf = [0u8; 4096];

        loop {
            match self.next_event(master, child_pid, &mut buf) {
                ServerEvent::Output(data) => {
                    self.vt.feed_str(&String::from_utf8_lossy(data));
                    self.clients.retain_mut(|c| c.write_all(data).is_ok());
                }
                ServerEvent::ClientInput(data) => {
                    let _ = master.write_all(&data);
                }
                ServerEvent::ClientResize(rows, cols) => {
                    set_window_size(master, rows, cols);
                    self.vt.resize(cols as usize, rows as usize);
                }
                ServerEvent::Done => break,
            }
        }
    }

    fn next_event<'a>(
        &mut self,
        master: &mut File,
        child_pid: i32,
        buf: &'a mut [u8],
    ) -> ServerEvent<'a> {
        loop {
            if has_child_exited(child_pid) {
                return ServerEvent::Done;
            }

            let mut poll_set = PollSet::default();
            let master_idx = poll_set.add(master);
            let listener_idx = poll_set.add(&self.listener);
            let client_indices: Vec<_> = self.clients.iter().map(|c| poll_set.add(c)).collect();

            match poll_set.wait(500) {
                PollResult::Interrupted | PollResult::Timeout => continue,
                PollResult::Error => return ServerEvent::Done,
                PollResult::Ready => {}
            }

            if poll_set.is_readable(master_idx) {
                return match master.read(buf) {
                    Ok(0) | Err(_) => ServerEvent::Done,
                    Ok(n) => ServerEvent::Output(&buf[..n]),
                };
            }
            if poll_set.is_hungup(master_idx) {
                return ServerEvent::Done;
            }
            if poll_set.is_readable(listener_idx) {
                self.accept(master);
            }

            // Check each client for data
            for (i, &idx) in client_indices.iter().enumerate() {
                if poll_set.is_readable(idx) || poll_set.is_hungup(idx) {
                    match Self::read_client_message(&mut self.clients[i], &mut self.client_buf) {
                        Message::Input(data) => return ServerEvent::ClientInput(data),
                        Message::Resize { rows, cols } => {
                            return ServerEvent::ClientResize(rows, cols);
                        }
                        Message::DumpText => {
                            for line in self.vt.text() {
                                if !line.is_empty() {
                                    let _ = self.clients[i].write_all(line.as_bytes());
                                    let _ = self.clients[i].write_all(b"\r\n");
                                }
                            }
                        }
                        Message::Detach => {
                            self.clients.remove(i);
                            break;
                        }
                    }
                }
            }
        }
    }

    fn accept(&mut self, master: &File) {
        let Ok((mut stream, _)) = self.listener.accept() else {
            return;
        };

        // Read the initial resize (blocking, before set_nonblocking)
        let mut buf = [0u8; 5];
        if let Ok(n) = stream.read(&mut buf)
            && let Message::Resize { rows, cols } = Message::decode(&buf[..n])
        {
            set_window_size(master, rows, cols);
            self.vt.resize(cols as usize, rows as usize);
        }

        // Send the current screen state before joining the broadcast list.
        // Otherwise live PTY output can reach the client before the dump,
        // and the dump's cursor escapes smear that output.
        let screen = self.vt.dump().replace('\u{9b}', "\x1b[");
        let _ = stream.write_all(screen.as_bytes());

        stream.set_nonblocking(true).ok();
        self.clients.push(stream);
    }

    fn read_client_message(client: &mut UnixStream, buf: &mut [u8]) -> Message {
        match client.read(buf) {
            Ok(0) | Err(_) => Message::Detach,
            Ok(n) => Message::decode(&buf[..n]),
        }
    }
}
