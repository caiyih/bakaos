use abstractions::IUsizeAlias;
use address::{IAddressBase, IAlignableAddress, VirtualAddress};
use constants::ErrNo;
use linux_loader::auxv::AuxVecValues;
use linux_loader::{IExecSource, LinuxLoader, ProcessContext, RawMemorySpace};
use platform_specific::ITaskContext;
use platform_specific::TaskTrapContext;
use stream::{IMMUStreamExt, MemoryStream};
use task_abstractions::status::TaskStatus;
use utilities::InvokeOnDrop;

use crate::{SyscallContext, SyscallResult};

impl SyscallContext {
    const ARRAY_MAX_LEN: usize = 1024; // temporary value
    const STRING_MAX_LEN: usize = 4096; // temporary value

    /// The `execve` system call implementation.
    /// This syscall build a new memory space wtih the given executable file,
    /// initializing the new memory space's stack with the given arguments and environment variables,
    /// and replace current process's memory space with the new one.
    pub fn sys_execve(
        &self,
        pathname: VirtualAddress,
        argv: VirtualAddress,
        envp: VirtualAddress,
    ) -> SyscallResult {
        let process = self.task.linux_process();

        let mmu = process.mmu();
        let locked_mmu = mmu.lock();
        let mut stream = locked_mmu.create_stream(VirtualAddress::null(), true);

        let path = import_c_bytes(&mut stream, pathname, Self::STRING_MAX_LEN)?;
        let path = core::str::from_utf8(path).map_err(|_| ErrNo::InvalidArgument)?;

        let argv_pointers = import_c_ptr_array(&mut stream, argv, Self::ARRAY_MAX_LEN)?;
        let argv_contents = import_c_bytes_array(&mut stream, argv_pointers, Self::STRING_MAX_LEN)?;

        let envp_pointers = import_c_ptr_array(&mut stream, envp, Self::ARRAY_MAX_LEN)?;
        let envp_contents = import_c_bytes_array(&mut stream, envp_pointers, Self::STRING_MAX_LEN)?;

        core::mem::forget(stream); // prevent mapped buffer from being dropped and releasing borrow from mmu

        // Free the buffers manually, this will be called when it was dropped
        let _guard = InvokeOnDrop::new(|_| {
            let free = |vaddr: VirtualAddress| locked_mmu.unmap_buffer(vaddr);
            let free_array = |array: &[VirtualAddress]| {
                for &vaddr in array {
                    debug_assert!(!vaddr.is_null());

                    free(vaddr);
                }
            };

            free(pathname);
            free_array(argv_pointers);
            free_array(envp_pointers);
            free(argv);
            free(envp);
        });

        self.sys_execve_internal(
            [0u8].as_slice(), // TODO
            path,
            &argv_contents,
            &envp_contents,
            // TODO: pass the locked mmu, since we can't unlock it until the execve is done
            //otherwise the memory may be invalid due to modification to the memory space
        )
    }

    /// Perform an execve-like replacement of the current task's address space with a new executable.
    ///
    /// Attempts to load `executable` at `pathname` into a fresh memory space, replace the process's
    /// memory space with the loaded image, initialize the task's trap context (entry PC, stack top,
    /// argv/envp bases and argc), and mark the task Ready. On loader failure this returns
    /// `ErrNo::ExecFormatError`.
    ///
    /// Note: argv and envp parameters are accepted by this function but are currently not wired into
    /// the loader (FIXME). Auxv values are also supplied as defaults (TODO: populate machine info).
    ///
    /// Parameters:
    /// - `executable`: an object implementing `IExecSource` that provides the raw executable bytes.
    /// - `pathname`: the path string used for loader semantics and /proc visibility.
    /// - `argv`: program arguments (currently not forwarded to the loader).
    /// - `envp`: environment variables (currently not forwarded to the loader).
    ///
    /// Returns:
    /// - `Ok(0)` on success.
    /// - `Err(ErrNo::ExecFormatError)` if the loader rejects the executable format.
    ///
    /// Side effects:
    /// - Replaces the process memory space via `process.execve(...)`.
    /// - Updates the task's trap context and status to `TaskStatus::Ready`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Given a `ctx: SyscallContext`, an executable `exe` and path:
    /// let _ = ctx.sys_execve_internal(exe, "/bin/app", &["app", "--help"], &[]);
    /// ```
    fn sys_execve_internal(
        &self,
        executable: impl IExecSource,
        pathname: &str,
        _argv: &[&[u8]],
        _envp: &[&[u8]],
    ) -> SyscallResult {
        let process = self.task.linux_process();

        let (mmu, alloc) = {
            let mem = process.memory_space().lock();
            (mem.mmu().clone(), mem.allocator().clone())
        };

        let process_ctx = ProcessContext::new();

        // FIXME: Pass argv, envp

        // TODO: resolve machine's information and pass it to auxv

        let memory_space: RawMemorySpace = (mmu, alloc); // FIXME: should be the new process's

        let loader = LinuxLoader::from_raw(
            &executable,
            pathname,
            process_ctx,
            AuxVecValues::default(), // TODO: populate machine info
            self.kernel.fs().lock().clone(),
            &memory_space,
            None, // FIXME: should be the calling thread's
        )
        .map_err(|_| ErrNo::ExecFormatError)?;

        let calling_thread = self.task.tid();

        process.execve(loader.memory_space, calling_thread);

        let trap_ctx = TaskTrapContext::new(
            loader.entry_pc.as_usize(),
            loader.stack_top.as_usize(),
            loader.ctx.argv.len(),
            loader.argv_base.as_usize(),
            loader.envp_base.as_usize(),
        );

        self.task.trap_context_mut().copy_from(&trap_ctx);

        self.task.update_status(TaskStatus::Ready);

        Ok(0)
    }
}

/// The Rust's borrow checker bind the lifetime with ownership
/// This function promotes the lifetime to 'static to unbind ownership from the lifetime
///
/// # Safety
/// The caller must ensure the slice is valid in the lifetime of the returned static slice.
/// In our case, must be dropped before the mmu is unlocked.
unsafe fn bump_slice_to_static<T>(val: &[T]) -> &'static [T] {
    unsafe { core::slice::from_raw_parts(val.as_ptr(), val.len()) }
}

/// Import a C-style pointer array from given memory space via stream.
fn import_c_ptr_array(
    stream: &mut MemoryStream,
    ptr: VirtualAddress,
    max_len: usize,
) -> Result<&'static [VirtualAddress], ErrNo> {
    if !ptr.is_aligned(core::mem::align_of::<VirtualAddress>()) {
        return Err(ErrNo::InvalidArgument);
    }

    let max_idx = max_len - 1;

    stream.seek(stream::Whence::Set(ptr));

    let mut read_complete = false;

    let slice = stream
        .read_unsized_slice::<VirtualAddress>(|ptr, idx| {
            if ptr.is_null() {
                read_complete = true;
                return false;
            }

            idx < max_idx
        })
        .map_err(|_| ErrNo::BadAddress)?;

    if !read_complete {
        // TODO: too long
        return Err(ErrNo::InvalidArgument);
    }

    Ok(unsafe { bump_slice_to_static(slice) })
}

/// Import a C-style bytes array from given memory space via stream.
fn import_c_bytes(
    stream: &mut MemoryStream,
    ptr: VirtualAddress,
    max_len: usize,
) -> Result<&'static [u8], ErrNo> {
    let max_idx = max_len - 1;

    stream.seek(stream::Whence::Set(ptr));

    let mut read_complete = false;
    let slice = stream
        .read_unsized_slice::<u8>(|byte, idx| {
            if *byte == 0 {
                read_complete = true;
                return false;
            }

            idx < max_idx
        })
        .map_err(|_| ErrNo::BadAddress)?;

    if !read_complete {
        // TODO: too long
        return Err(ErrNo::InvalidArgument);
    }

    Ok(unsafe { bump_slice_to_static(slice) })
}

/// Import an array of C-style bytes array from given memory space via stream.
fn import_c_bytes_array(
    stream: &mut MemoryStream,
    array: &[VirtualAddress],
    content_max_len: usize,
) -> Result<Vec<&'static [u8]>, ErrNo> {
    let mut result = Vec::with_capacity(array.len());

    for &ptr in array {
        let slice = import_c_bytes(stream, ptr, content_max_len)?;
        result.push(slice);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {}
