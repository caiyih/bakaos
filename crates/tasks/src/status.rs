#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskStatus {
    Uninitialized = 0,
    Ready = 1,
    Running = 2,
    Exited = 4,
    Zombie = 8,
}
