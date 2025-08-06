use rustix::{
    fs::FlockOperation,
    io::{retry_on_intr, IoSlice, IoSliceMut},
    net,
};
use std::{
    collections::VecDeque,
    fs, io, ops,
    os::unix::{
        fs::OpenOptionsExt,
        io::{AsFd, BorrowedFd, OwnedFd},
        net::UnixStream,
    },
    path::PathBuf,
};

// Panics if iterator isn't as long as `N`
pub fn array_from_iterator_unchecked<T: Copy + Default, I: Iterator<Item = T>, const N: usize>(
    mut iter: I,
) -> [T; N] {
    let mut arr = [T::default(); N];
    for i in &mut arr {
        *i = iter.next().unwrap();
    }
    arr
}

pub fn send_with_fds(
    socket: &UnixStream,
    //buf: &VecDeque<u8>,
    buf: &[IoSlice],
    fds: &[BorrowedFd],
) -> rustix::io::Result<usize> {
    #[allow(clippy::manual_slice_size_calculation)]
    let mut cmsg_space = vec![0; rustix::cmsg_space!(ScmRights(fds.len()))];
    let mut cmsg_buffer = net::SendAncillaryBuffer::new(&mut cmsg_space);
    cmsg_buffer.push(net::SendAncillaryMessage::ScmRights(fds));
    retry_on_intr(|| net::sendmsg(socket, buf, &mut cmsg_buffer, net::SendFlags::NOSIGNAL))
}

pub fn recv_with_fds(
    socket: &UnixStream,
    buf: &mut [u8],
    fds: &mut VecDeque<OwnedFd>,
) -> rustix::io::Result<usize> {
    const MAX_FDS: usize = 32;

    #[allow(clippy::manual_slice_size_calculation)]
    let mut cmsg_space = vec![0; rustix::cmsg_space!(ScmRights(MAX_FDS))];
    let mut cmsg_buffer = net::RecvAncillaryBuffer::new(&mut cmsg_space);
    let response = retry_on_intr(|| {
        net::recvmsg(
            socket,
            &mut [IoSliceMut::new(buf)],
            &mut cmsg_buffer,
            net::RecvFlags::CMSG_CLOEXEC,
        )
    })?;
    if response.bytes != 0 {
        fds.extend(
            cmsg_buffer
                .drain()
                .filter_map(|msg| match msg {
                    net::RecvAncillaryMessage::ScmRights(fds) => Some(fds),
                    _ => None,
                })
                .flatten(),
        );
    }
    Ok(response.bytes)
}

#[derive(Debug)]
pub struct UnlinkOnDrop<T> {
    inner: T,
    path: PathBuf,
}

impl<T> UnlinkOnDrop<T> {
    pub fn new(inner: T, path: PathBuf) -> Self {
        Self { inner, path }
    }
}

impl<T> Drop for UnlinkOnDrop<T> {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl<T> ops::Deref for UnlinkOnDrop<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> ops::DerefMut for UnlinkOnDrop<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

// Should match way locking in libeis is handled
#[derive(Debug)]
pub struct LockFile(#[allow(dead_code)] UnlinkOnDrop<fs::File>);

impl LockFile {
    pub fn new(path: PathBuf) -> io::Result<Option<Self>> {
        let inner = fs::File::options()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .mode(0o660)
            .open(&path)?;
        let inner = UnlinkOnDrop::new(inner, path);
        let locked =
            rustix::fs::flock(&inner.inner, FlockOperation::NonBlockingLockExclusive).is_ok();
        Ok(Some(inner).filter(|_| locked).map(Self))
    }
}

pub fn poll_readable<T: AsFd>(fd: &T) -> io::Result<()> {
    rustix::io::retry_on_intr(|| {
        rustix::event::poll(
            &mut [rustix::event::PollFd::new(fd, rustix::event::PollFlags::IN)],
            0,
        )
    })?;
    Ok(())
}
