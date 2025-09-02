use allocation_abstractions::IFrameAllocator;
use filesystem_abstractions::DirectoryTreeNode;
use hermit_sync::SpinMutex;
use kernel_abstractions::{IKernel, IKernelSerial};
use std::{
    collections::vec_deque::VecDeque,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
    vec::Vec,
};
use timing::TimeSpec;

pub struct TestKernel {
    pub serial: Option<Arc<dyn IKernelSerial>>,
    pub fs: Option<Arc<SpinMutex<Arc<DirectoryTreeNode>>>>,
    pub allocator: Option<Arc<SpinMutex<dyn IFrameAllocator>>>,
}

unsafe impl Send for TestKernel {}
unsafe impl Sync for TestKernel {}

impl Default for TestKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl TestKernel {
    pub fn new() -> Self {
        Self {
            serial: None,
            fs: None,
            allocator: None,
        }
    }

    pub fn with_serial(mut self, serial: Option<Arc<impl IKernelSerial>>) -> Self {
        self.serial = serial.map(|s| s as Arc<dyn IKernelSerial>);
        self
    }

    pub fn with_fs(mut self, fs: Option<Arc<DirectoryTreeNode>>) -> Self {
        self.fs = fs.map(|f| Arc::new(SpinMutex::new(f)));
        self
    }

    pub fn with_allocator(mut self, alloc: Option<Arc<SpinMutex<dyn IFrameAllocator>>>) -> Self {
        self.allocator = alloc;
        self
    }

    pub fn build(self) -> Arc<dyn IKernel> {
        Arc::new(self)
    }
}

impl IKernel for TestKernel {
    fn serial(&self) -> Arc<dyn IKernelSerial> {
        self.serial.as_ref().unwrap().clone()
    }

    fn fs(&self) -> Arc<SpinMutex<Arc<DirectoryTreeNode>>> {
        self.fs.as_ref().unwrap().clone()
    }

    fn allocator(&self) -> Arc<SpinMutex<dyn IFrameAllocator>> {
        self.allocator.as_ref().unwrap().clone()
    }

    fn activate_mmu(&self, _pt: &dyn mmu_abstractions::IMMU) {}

    fn time(&self) -> TimeSpec {
        let now = SystemTime::now();
        let unix = now.duration_since(UNIX_EPOCH).unwrap();
        TimeSpec {
            tv_sec: unix.as_secs() as i64,
            tv_nsec: unix.subsec_nanos() as i64,
        }
    }
}

pub struct TestSerial {
    pub output: SpinMutex<Vec<u8>>,
    pub input: SpinMutex<VecDeque<u8>>,
}

impl Default for TestSerial {
    fn default() -> Self {
        Self::new()
    }
}

impl TestSerial {
    pub fn new() -> Self {
        Self {
            output: SpinMutex::new(Vec::new()),
            input: SpinMutex::new(VecDeque::new()),
        }
    }

    pub fn input(&self, bytes: &[u8]) {
        let mut input = self.input.lock();

        for c in bytes {
            input.push_back(*c);
        }
    }

    pub fn content(&self) -> Vec<u8> {
        self.output.lock().clone()
    }
}

impl IKernelSerial for TestSerial {
    fn send(&self, byte: u8) -> Result<(), &'static str> {
        self.output.lock().push(byte);

        Ok(())
    }

    fn recv(&self) -> Option<u8> {
        self.input.lock().pop_front()
    }
}
