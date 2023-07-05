use rustix::{
    io::{retry_on_intr, IoSlice, IoSliceMut},
    net,
};
use std::{
    collections::VecDeque,
    os::unix::{
        io::{BorrowedFd, OwnedFd},
        net::UnixStream,
    },
};

// Panics if iterator isn't as long as `N`
pub fn array_from_iterator_unchecked<T: Copy + Default, I: Iterator<Item = T>, const N: usize>(
    mut iter: I,
) -> [T; N] {
    let mut arr = [T::default(); N];
    for i in 0..N {
        arr[i] = iter.next().unwrap();
    }
    arr
}

pub fn send_with_fds(
    socket: &UnixStream,
    buf: &[u8],
    fds: &[BorrowedFd],
) -> rustix::io::Result<()> {
    let mut cmsg_space = vec![0; rustix::cmsg_space!(ScmRights(fds.len()))];
    let mut cmsg_buffer = net::SendAncillaryBuffer::new(&mut cmsg_space);
    cmsg_buffer.push(net::SendAncillaryMessage::ScmRights(fds));
    retry_on_intr(|| {
        net::sendmsg(
            socket,
            &[IoSlice::new(buf)],
            &mut cmsg_buffer,
            net::SendFlags::NOSIGNAL,
        )
    })?;
    Ok(())
}

pub fn recv_with_fds(
    socket: &UnixStream,
    buf: &mut [u8],
    fds: &mut VecDeque<OwnedFd>,
) -> rustix::io::Result<usize> {
    const MAX_FDS: usize = 32;

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
