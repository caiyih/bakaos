use core::{
    future::Future,
    mem::MaybeUninit,
    ptr,
    sync::atomic::Ordering,
    task::{Poll, Waker},
};

use alloc::{
    collections::btree_map::BTreeMap,
    format,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use drivers::{current_timespec, ITimer};
use filesystem_abstractions::{
    global_mount_inode, DirectoryEntry, DirectoryEntryType, FileStatistics, FileStatisticsMode,
    FileSystemError, FileSystemResult, IInode, InodeMetadata,
};
use hermit_sync::SpinMutex;
use log::debug;
use platform_abstractions::return_to_user;
use tasks::{TaskControlBlock, TaskStatus};
use timing::TimeSpec;

use crate::{processor::ProcessorUnit, trap::user_trap_handler_async};

struct ExposeWakerFuture;

impl Future for ExposeWakerFuture {
    type Output = Waker;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        ctx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        Poll::Ready(ctx.waker().clone())
    }
}

#[no_mangle]
// Complete lifecycle of a task from ready to exited
async fn task_loop(tcb: Arc<TaskControlBlock>) {
    debug_assert!(
        tcb.is_ready(),
        "task must be ready to run, but got {:?}",
        tcb.task_status
    );

    unsafe {
        // We can't pass the waker(or the context) to nested functions, so we store it in the tcb.
        *tcb.waker.get() = MaybeUninit::new(ExposeWakerFuture.await);
        *tcb.start_time.get().as_mut().unwrap() = MaybeUninit::new(current_timespec());
    }

    *tcb.task_status.lock() = TaskStatus::Running;
    add_to_map(&tcb);

    while !tcb.is_exited() {
        let return_reason = return_to_user(unsafe { tcb.mut_trap_ctx() });
        tcb.kernel_timer.lock().start();

        // Returned from user program. Entering trap handler.
        // We've actually saved the trap context before returned from `return_to_user`.

        debug_assert!(tcb.is_running(), "task should be running");

        user_trap_handler_async(&tcb, return_reason).await;

        tcb.kernel_timer.lock().set();
    }

    debug!(
        "Task {} has completed its lifecycle with code: {}, cleaning up...",
        tcb.task_id.id(),
        tcb.exit_code.load(Ordering::Relaxed)
    );

    remove_from_map(&tcb);

    let cpu = ProcessorUnit::current();
    assert!(Arc::ptr_eq(&cpu.staged_task().unwrap(), &tcb));
    cpu.pop_staged_task();

    // Some cleanup, like dangling child tasks, etc.
}

struct TaskFuture<F: Future + Send + 'static> {
    tcb: Arc<TaskControlBlock>,
    fut: F,
}

impl<TFut: Future + Send + 'static> TaskFuture<TFut> {
    fn new(tcb: Arc<TaskControlBlock>, fut: TFut) -> Self {
        Self {
            tcb: tcb.clone(),
            fut,
        }
    }
}

impl<TFut: Future + Send + 'static> Future for TaskFuture<TFut> {
    type Output = TFut::Output;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        ctx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let cpu = ProcessorUnit::current();
        cpu.stage_task(self.tcb.clone());

        self.tcb.timer.lock().start();

        let task_fut = unsafe { self.as_mut().map_unchecked_mut(|s| &mut s.fut) };
        let ret = task_fut.poll(ctx);

        self.tcb.timer.lock().set();

        ret
    }
}

static TASKS_MAP: SpinMutex<BTreeMap<usize, Weak<TaskControlBlock>>> =
    SpinMutex::new(BTreeMap::new());

fn add_to_map(tcb: &Arc<TaskControlBlock>) {
    let previous = TASKS_MAP
        .lock()
        .insert(tcb.task_id.id(), Arc::downgrade(tcb));

    debug_assert!(previous.is_none());
}

fn remove_from_map(tcb: &Arc<TaskControlBlock>) {
    let removed = TASKS_MAP.lock().remove(&tcb.task_id.id());

    debug_assert!(removed.is_some());
    debug_assert!(ptr::addr_eq(
        Arc::as_ptr(tcb),
        Weak::as_ptr(&removed.unwrap())
    ))
    // Arc::ptr_eq(this, other)
}

pub fn task_count() -> usize {
    TASKS_MAP.lock().len()
}

#[allow(unused)]
pub fn get_task(tid: usize) -> Option<Arc<TaskControlBlock>> {
    unsafe { TASKS_MAP.lock().get(&tid).and_then(|weak| weak.upgrade()) }
}

#[allow(unused)]
pub fn spawn_task(tcb: Arc<TaskControlBlock>) {
    tcb.init();
    let fut = TaskFuture::new(tcb.clone(), task_loop(tcb));
    threading::spawn(fut);
}

pub struct ProcDeviceInode;

impl ProcDeviceInode {
    pub fn setup() {
        let proc: Arc<dyn IInode> = Arc::new(ProcDeviceInode);

        let proc = global_mount_inode(&proc, "/proc", None).unwrap();

        let self_link: Arc<dyn IInode> = Arc::new(SelfLinkInode);
        global_mount_inode(&self_link, "/proc/self", None).unwrap();

        proc.touch("mounts").unwrap();
        proc.touch("meminfo").unwrap();

        // TODO: add meminfo, cpu info...
    }
}

fn stat(stat: &mut FileStatistics, mode: FileStatisticsMode) -> FileSystemResult<()> {
    stat.device_id = 0;
    stat.inode_id = 0;
    stat.mode = mode;
    stat.link_count = 1;
    stat.uid = 0;
    stat.gid = 0;
    stat.size = 0;
    stat.block_size = 512;
    stat.block_count = 0;
    stat.rdev = 0;

    stat.ctime = TimeSpec::zero();
    stat.mtime = TimeSpec::zero();
    stat.atime = TimeSpec::zero();

    Ok(())
}

impl IInode for ProcDeviceInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "proc",
            entry_type: DirectoryEntryType::Directory,
            size: 0,
        }
    }

    fn lookup(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        if let Ok(tid) = name.parse::<usize>() {
            if let Some(tcb) = TASKS_MAP.lock().get(&tid) {
                if tcb.strong_count() > 0 {
                    return Ok(Arc::new(ProcessDirectoryInode(tcb.clone())));
                }
            }
        }

        Err(FileSystemError::NotFound)
    }

    fn read_cache_dir(
        &self,
        _caches: &mut BTreeMap<String, Arc<dyn IInode>>, // not needed
    ) -> FileSystemResult<Vec<DirectoryEntry>> {
        let tasks = TASKS_MAP.lock();

        let mut entries = Vec::with_capacity(tasks.len());

        for task in tasks.iter().filter_map(|(_, w)| w.upgrade()) {
            entries.push(DirectoryEntry {
                filename: task.task_id.id().to_string(),
                entry_type: DirectoryEntryType::Directory,
            });
        }

        // release unnecessary memory
        entries.shrink_to_fit();

        Ok(entries)
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        self::stat(stat, FileStatisticsMode::DIR)
    }
}

struct SelfLinkInode;

impl IInode for SelfLinkInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "self",
            entry_type: DirectoryEntryType::Symlink,
            size: 0,
        }
    }

    fn resolve_link(&self) -> Option<String> {
        ProcessorUnit::current()
            .staged_task()
            .map(|t| format!("{}/", t.task_id.id()))
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        self::stat(stat, FileStatisticsMode::LINK)
    }
}

struct ProcessDirectoryInode(Weak<TaskControlBlock>);

impl IInode for ProcessDirectoryInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "",
            entry_type: DirectoryEntryType::Directory,
            size: 0,
        }
    }

    fn read_cache_dir(
        &self,
        _caches: &mut BTreeMap<String, Arc<dyn IInode>>,
    ) -> FileSystemResult<Vec<DirectoryEntry>> {
        if self.0.strong_count() > 0 {
            #[inline]
            fn entry(name: &str, entry_type: DirectoryEntryType) -> DirectoryEntry {
                DirectoryEntry {
                    filename: String::from(name),
                    entry_type,
                }
            }

            Ok(vec![
                entry("exe", DirectoryEntryType::Symlink),
                entry("cwd", DirectoryEntryType::Symlink),
            ])
        } else {
            Err(FileSystemError::NotFound)
        }
    }

    fn lookup(&self, name: &str) -> FileSystemResult<Arc<dyn IInode>> {
        if let Some(tcb) = self.0.upgrade() {
            if name == "exe" {
                return Ok(Arc::new(LinkToInode(
                    tcb.pcb.lock().executable.as_ref().clone(),
                )));
            }

            if name == "cwd" {
                return Ok(Arc::new(LinkToInode(tcb.pcb.lock().cwd.clone())));
            }
        }

        Err(FileSystemError::NotFound)
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        match self.0.strong_count() {
            0 => Err(FileSystemError::NotFound),
            _ => self::stat(stat, FileStatisticsMode::DIR),
        }
    }
}

struct LinkToInode(String);

impl IInode for LinkToInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: "",
            entry_type: DirectoryEntryType::Symlink,
            size: 0,
        }
    }

    fn resolve_link(&self) -> Option<String> {
        Some(self.0.clone())
    }

    fn stat(&self, stat: &mut FileStatistics) -> FileSystemResult<()> {
        self::stat(stat, FileStatisticsMode::LINK)
    }
}
