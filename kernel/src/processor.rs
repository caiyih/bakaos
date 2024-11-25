use core::{arch::asm, mem::MaybeUninit, usize};

use alloc::sync::Arc;
use tasks::TaskControlBlock;

#[allow(unused)]
pub struct ProcessorUnit {
    hart_id: usize,
    staged_task: Option<Arc<TaskControlBlock>>,
}

impl Default for ProcessorUnit {
    fn default() -> Self {
        ProcessorUnit {
            hart_id: usize::MAX,
            staged_task: None,
        }
    }
}

#[allow(unused)]
impl ProcessorUnit {
    pub fn new(hart_id: usize) -> Self {
        ProcessorUnit {
            hart_id,
            staged_task: None,
        }
    }

    #[no_mangle]
    pub fn stage_task(&mut self, task: Arc<TaskControlBlock>) {
        unsafe { task.memory_space.lock().page_table().activate() };
        self.staged_task = Some(task);

        // TODO: setup timer
    }

    pub fn staged_task(&self) -> Option<Arc<TaskControlBlock>> {
        self.staged_task.clone()
    }

    pub fn pop_staged_task(&mut self) -> Option<Arc<TaskControlBlock>> {
        let tcb = self.staged_task.take();
        // TODO: stop timer

        // When popping the staged task and the task has exited, the tcb will soon be dropped.
        // which means the page table will be dropped as well. So we need to use another page table.
        // Since it's hard to determine whether there will be new tasks staged in the future,
        // or whether the old page table will be used again, we just use the kernel's page table.
        unsafe { paging::page_table::get_kernel_page_table().activate() };

        tcb
    }

    pub fn is_idle(&self) -> bool {
        self.staged_task.is_none()
    }
}

#[allow(unused)]
impl ProcessorUnit {
    pub fn hart_id(&self) -> usize {
        self.hart_id
    }

    pub fn is_current(&self) -> bool {
        self.hart_id == hart_id()
    }

    pub fn current() -> &'static mut Self {
        current_processor()
    }
}

pub const PROCESSOR_COUNT: usize = 2;

static mut PROCESSOR_POOL: MaybeUninit<[ProcessorUnit; PROCESSOR_COUNT]> = MaybeUninit::uninit();

pub fn init_processor_pool() {
    let pool = unsafe { PROCESSOR_POOL.assume_init_mut() };
    for (i, cpu) in pool.iter_mut().enumerate().take(PROCESSOR_COUNT) {
        *cpu = ProcessorUnit::new(i);
    }
}

#[inline(always)]
pub fn hart_id() -> usize {
    let id;

    unsafe {
        asm!("mv {}, tp", out(reg) id, options(nostack, nomem));
    }

    id
}

pub fn current_processor() -> &'static mut ProcessorUnit {
    let id = hart_id();
    unsafe { &mut PROCESSOR_POOL.assume_init_mut()[id] }
}
