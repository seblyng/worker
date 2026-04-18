use std::{
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
};

pub struct Pty {
    pub master: File,
    pub slave: File,
}

impl Pty {
    pub fn open(rows: u16, cols: u16) -> Result<Self, anyhow::Error> {
        let master = unsafe {
            let fd = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            anyhow::ensure!(fd >= 0, "posix_openpt failed");
            File::from_raw_fd(fd)
        };

        unsafe {
            anyhow::ensure!(libc::grantpt(master.as_raw_fd()) == 0, "grantpt failed");
            anyhow::ensure!(libc::unlockpt(master.as_raw_fd()) == 0, "unlockpt failed");
            let slave_name = libc::ptsname(master.as_raw_fd());
            anyhow::ensure!(!slave_name.is_null(), "ptsname failed");
            let fd = libc::open(slave_name, libc::O_RDWR);
            anyhow::ensure!(fd >= 0, "failed to open slave pty");
            let slave = File::from_raw_fd(fd);

            set_window_size(&master, rows, cols);

            Ok(Pty { master, slave })
        }
    }
}

pub fn set_window_size(fd: &File, rows: u16, cols: u16) {
    let ws = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe { libc::ioctl(fd.as_raw_fd(), libc::TIOCSWINSZ, &ws) };
}

pub fn get_terminal_size() -> (u16, u16) {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 {
            (ws.ws_row, ws.ws_col)
        } else {
            (24, 80)
        }
    }
}

/// Protocol messages between PtyClient and PtyServer.
pub enum Message {
    /// Resize the PTY.
    Resize { rows: u16, cols: u16 },
    /// Request the current screen state as plain text.
    DumpText,
    /// Detach from the session.
    Detach,
    /// Forward input to the PTY.
    Input(Vec<u8>),
}

impl Message {
    const RESIZE: u8 = 0x01;
    const DUMP_TEXT: u8 = 0x03;
    const CTRL_D: u8 = 0x04;

    pub fn encode(&self) -> Vec<u8> {
        match self {
            Message::Resize { rows, cols } => {
                let mut buf = vec![Self::RESIZE];
                buf.extend_from_slice(&rows.to_be_bytes());
                buf.extend_from_slice(&cols.to_be_bytes());
                buf
            }
            Message::DumpText => vec![Self::DUMP_TEXT],
            Message::Detach => vec![Self::CTRL_D],
            Message::Input(data) => data.clone(),
        }
    }

    pub fn decode(buf: &[u8]) -> Self {
        if buf.contains(&Self::CTRL_D) {
            return Message::Detach;
        }
        if buf.len() == 5 && buf[0] == Self::RESIZE {
            let rows = u16::from_be_bytes([buf[1], buf[2]]);
            let cols = u16::from_be_bytes([buf[3], buf[4]]);
            return Message::Resize { rows, cols };
        }
        if buf.len() == 1 && buf[0] == Self::DUMP_TEXT {
            return Message::DumpText;
        }
        Message::Input(buf.to_vec())
    }
}

/// Enter raw mode if stdin is a terminal. Returns None if stdin is not a terminal.
pub fn enter_raw_mode() -> Option<libc::termios> {
    unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 0 {
            return None;
        }
        let mut original: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &mut original) != 0 {
            return None;
        }
        let mut raw = original;
        libc::cfmakeraw(&mut raw);
        if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) != 0 {
            return None;
        }
        // Switch to alternate screen so avt.dump()'s absolute cursor
        // positioning lands on a fresh (1,1)-based buffer instead of
        // overlapping the shell history.
        let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[?1049h\x1b[H");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        Some(original)
    }
}

pub fn restore_terminal(original: &libc::termios) {
    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, original);
    }
    // Reset terminal state that escape sequences (not termios) control,
    // e.g. cursor visibility, alternate screen buffer
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[?25h\x1b[?1049l");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}
