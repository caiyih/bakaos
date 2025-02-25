use constants::SyscallError;
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use threading::yield_now;
use timing::TimeSpec;

use crate::{async_syscall, timing::current_timespec};

const FD_SETSIZE: usize = 1024;
const NFD_PER_USIZE: usize = core::mem::size_of::<usize>() * 8;

#[repr(C)]
#[derive(Default, Clone)]
struct FdSet {
    bits: [usize; FD_SETSIZE / NFD_PER_USIZE],
}

impl FdSet {
    pub fn insert(&mut self, fd: usize) {
        if fd < FD_SETSIZE {
            let usize_index = fd / NFD_PER_USIZE;
            let bit_index = fd % NFD_PER_USIZE;
            self.bits[usize_index] |= 1 << bit_index;
        }
    }

    #[inline]
    #[allow(unused)]
    pub fn remove(&mut self, fd: usize) {
        if fd < FD_SETSIZE {
            let usize_index = fd / NFD_PER_USIZE;
            let bit_index = fd % NFD_PER_USIZE;
            self.bits[usize_index] &= !(1 << bit_index);
        }
    }

    #[inline]
    pub fn contains(&self, fd: usize) -> bool {
        if fd < FD_SETSIZE {
            let usize_index = fd / NFD_PER_USIZE;
            let bit_index = fd % NFD_PER_USIZE;
            (self.bits[usize_index] & (1 << bit_index)) != 0
        } else {
            false
        }
    }

    #[allow(unused)]
    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    pub fn iter(&self, nfds: usize) -> FdSetIterator {
        FdSetIterator {
            fd_set: self,
            current: 0,
            nfds,
        }
    }
}

pub struct FdSetIterator<'a> {
    fd_set: &'a FdSet,
    current: usize,
    nfds: usize,
}

impl Iterator for FdSetIterator<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.nfds {
            if self.fd_set.contains(self.current) {
                self.current += 1;
                return Some(self.current - 1);
            }
            self.current += 1;
        }
        None
    }
}

async_syscall!(sys_pselect6_async, ctx, {
    let nfds = ctx.arg0::<usize>();

    let pread_set = ctx.arg1::<*const FdSet>();
    let pwrite_set = ctx.arg1::<*const FdSet>();

    let pt = ctx.borrow_page_table();

    let mut readset = pt
        .guard_ptr(pread_set)
        .mustbe_user()
        .mustbe_readable()
        .with_read();

    let mut writeset = pt
        .guard_ptr(pwrite_set)
        .mustbe_user()
        .mustbe_readable()
        .with_read();

    let p_timeout = ctx.arg4::<*const TimeSpec>();
    let timeout = pt.guard_ptr(p_timeout).mustbe_user().with_read();
    let end_time = timeout.map(|t| *t + current_timespec());

    loop {
        let mut n_ready = 0;

        let mut readset_result = FdSet::default();
        let mut writeset_result = FdSet::default();
        {
            let pcb = ctx.pcb.lock();

            if let Some(ref mut readset) = readset.as_mut() {
                for fd_idx in readset.iter(nfds) {
                    if let Some(fd) = pcb.fd_table.get(fd_idx) {
                        if fd.can_read() {
                            let file = fd.access_ref();

                            if file.can_read() && file.read_avaliable() {
                                n_ready += 1;
                                readset_result.insert(fd_idx);
                            }
                        }
                    }
                }
            }

            if let Some(ref mut writeset) = writeset.as_mut() {
                for fd_idx in writeset.iter(nfds) {
                    if let Some(fd) = pcb.fd_table.get(fd_idx) {
                        if fd.can_write() {
                            let file = fd.access_ref();

                            if file.can_write() && file.write_avaliable() {
                                n_ready += 1;
                                writeset_result.insert(fd_idx);
                            }
                        }
                    }
                }
            }
        }

        if n_ready != 0 {
            if let Some(ref mut readset) = readset {
                *readset.as_mut() = readset_result;
            }

            if let Some(ref mut writeset) = writeset {
                *writeset.as_mut() = writeset_result;
            }

            return Ok(n_ready as isize);
        }

        if let Some(end_time) = end_time {
            if current_timespec() >= end_time {
                return Ok(0);
            }
        }

        yield_now().await;
    }
});

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

async_syscall!(sys_ppoll_async, ctx, {
    const POLLIN: i16 = 0x001;
    const POLLOUT: i16 = 0x004;

    let pfds = ctx.arg0::<*mut PollFd>();
    let nfds = ctx.arg1::<usize>();

    if pfds.is_null() {
        return SyscallError::BadAddress;
    }

    let fds = unsafe { core::slice::from_raw_parts_mut(pfds, nfds) };

    let pt = ctx.borrow_page_table();
    match pt
        .guard_slice(fds)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        Some(mut fds) => {
            let p_timeout = ctx.arg2::<*const TimeSpec>();
            let timeout = pt.guard_ptr(p_timeout).mustbe_user().with_read();

            let end_time = timeout.map(|t| *t + current_timespec());

            loop {
                let mut n_ready = 0;
                {
                    let pcb = ctx.pcb.lock();

                    for poll in fds.as_mut().iter_mut() {
                        poll.revents = 0;

                        if let Some(fd) = pcb.fd_table.get(poll.fd as usize) {
                            let file = fd.access_ref();
                            let mut ready = false;

                            // read
                            if poll.events & POLLIN == POLLIN
                                && fd.can_read()
                                && file.can_read()
                                && file.read_avaliable()
                            {
                                poll.revents |= POLLIN;
                                ready = true;
                            }

                            // write
                            if poll.events & POLLOUT == POLLOUT
                                && fd.can_write()
                                && file.can_write()
                                && file.write_avaliable()
                            {
                                poll.revents |= POLLOUT;
                                ready = true;
                            }

                            if ready {
                                n_ready += 1;
                            }
                        }
                    }
                }

                if n_ready > 0 {
                    return Ok(n_ready as isize);
                }

                if let Some(end_time) = end_time {
                    if current_timespec() >= end_time {
                        return Ok(0);
                    }
                }

                yield_now().await;
            }
        }
        None => return SyscallError::BadAddress,
    }
});
