use core::mem::MaybeUninit;

use alloc::sync::Arc;
use drivers::ITimer;
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
        unsafe { task.borrow_page_table().activate() };
        task.timer.lock().start();
        task.kernel_timer.lock().start();
        self.staged_task = Some(task);
    }

    pub fn staged_task(&self) -> Option<Arc<TaskControlBlock>> {
        self.staged_task.clone()
    }

    pub fn pop_staged_task(&mut self) -> Option<Arc<TaskControlBlock>> {
        let tcb = self.staged_task.take();

        if let Some(ref tcb) = tcb {
            tcb.timer.lock().set();
            tcb.kernel_timer.lock().set();
        }

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
        self.hart_id == platform_specific::current_processor_index()
    }

    pub fn current() -> &'static mut Self {
        current_processor()
    }
}

pub const PROCESSOR_COUNT: usize = 2;

static mut PROCESSOR_POOL: MaybeUninit<[ProcessorUnit; PROCESSOR_COUNT]> = MaybeUninit::uninit();

pub fn init_processor_pool() {
    #[allow(static_mut_refs)]
    let pool = unsafe { PROCESSOR_POOL.assume_init_mut() };
    for (i, cpu) in pool.iter_mut().enumerate().take(PROCESSOR_COUNT) {
        *cpu = ProcessorUnit::new(i);
    }
}

#[inline]
pub fn current_processor() -> &'static mut ProcessorUnit {
    let id = platform_specific::current_processor_index();
    unsafe {
        #[allow(static_mut_refs)]
        &mut PROCESSOR_POOL.assume_init_mut()[id]
    }
}
