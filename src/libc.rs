use std::os::fd::{AsRawFd, RawFd};

use serde::{Deserialize, Serialize};

pub enum Fork {
    Parent(libc::pid_t),
    Child,
}

pub fn fork() -> Result<Fork, i32> {
    let res = unsafe { libc::fork() };
    match res {
        -1 => Err(-1),
        0 => Ok(Fork::Child),
        res => Ok(Fork::Parent(res)),
    }
}

pub fn setsid() -> Result<libc::pid_t, i32> {
    let res = unsafe { libc::setsid() };
    match res {
        -1 => Err(-1),
        res => Ok(res),
    }
}

pub fn waitpid(pid: i32) -> Result<libc::pid_t, i32> {
    let mut status: i32 = 0;
    let res = unsafe { libc::waitpid(pid, &mut status, 0) };

    match res {
        -1 => Err(-1),
        res => Ok(res),
    }
}

/// Returns true if the child has exited.
pub fn has_child_exited(pid: i32) -> bool {
    let mut status: i32 = 0;
    unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) > 0 }
}

pub fn stop_pg(sid: i32, signal: &Signal) -> Result<(), i32> {
    match unsafe { libc::killpg(sid, signal.to_owned() as i32) } {
        0 => Ok(()),
        e => Err(e),
    }
}

pub fn dup2(src: i32, dst: i32) -> i32 {
    unsafe { libc::dup2(src, dst) }
}

pub fn signal(signum: i32, handler: usize) -> usize {
    unsafe { libc::signal(signum, handler) }
}

pub fn set_nonblocking(fd: RawFd) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

#[derive(Default)]
pub struct PollSet {
    fds: Vec<libc::pollfd>,
}

pub enum PollResult {
    Ready,
    Timeout,
    Interrupted,
    Error,
}

impl PollSet {
    pub fn add(&mut self, fd: &impl AsRawFd) -> usize {
        let idx = self.fds.len();
        self.fds.push(libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
        idx
    }

    pub fn wait(&mut self, timeout_ms: i32) -> PollResult {
        let ret = unsafe {
            libc::poll(
                self.fds.as_mut_ptr(),
                self.fds.len() as libc::nfds_t,
                timeout_ms,
            )
        };
        match ret {
            _ if ret > 0 => PollResult::Ready,
            0 => PollResult::Timeout,
            _ if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) => {
                PollResult::Interrupted
            }
            _ => PollResult::Error,
        }
    }

    pub fn is_readable(&self, index: usize) -> bool {
        self.fds[index].revents & libc::POLLIN != 0
    }

    pub fn is_hungup(&self, index: usize) -> bool {
        self.fds[index].revents & libc::POLLHUP != 0 && self.fds[index].revents & libc::POLLIN == 0
    }
}

#[derive(Deserialize, Clone, Debug, Serialize, Hash, PartialEq, Eq)]
#[non_exhaustive]
#[repr(i32)]
pub enum Signal {
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6,
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,
    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGSTKFLT = 16,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,
    SIGTTIN = 21,
    SIGTTOU = 22,
    SIGURG = 23,
    SIGXCPU = 24,
    SIGXFSZ = 25,
    SIGVTALRM = 26,
    SIGPROF = 27,
    SIGWINCH = 28,
    SIGIO = 29,
    SIGPWR = 30,
    SIGSYS = 31,
}
