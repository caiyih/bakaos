#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskStatus {
    /// The task is just created and has not been initialized to run.
    Uninitialized = 0,
    /// The task is ready to run.
    Ready = 1,
    /// The task is waiting for its CPU time slice.
    WaitingForScheduling = 2,
    /// The task is running on a CPU.
    Running = 4,
    /// The task is handling a signal event.
    HandlingSignal = 8,
    /// The task has exited.
    Exited = 16,
}

impl TaskStatus {
    pub fn is_ready(self) -> bool {
        self == Self::Ready
    }

    pub fn is_active(self) -> bool {
        self > Self::Ready
    }

    pub fn is_waiting(self) -> bool {
        self == Self::WaitingForScheduling
    }

    pub fn is_running(self) -> bool {
        self == Self::Running
    }

    pub fn is_handling_signal(self) -> bool {
        self == Self::HandlingSignal
    }

    pub fn is_exited(self) -> bool {
        self == Self::Exited
    }
}
