use alloc::string::String;
use filesystem_abstractions::{DirectoryEntryType, IInode, InodeMetadata};

pub struct SocketInode {
    name: String,
}

impl IInode for SocketInode {
    fn metadata(&self) -> InodeMetadata {
        InodeMetadata {
            filename: &self.name,
            entry_type: DirectoryEntryType::Socket,
            size: 0, // TODO: check this
        }
    }
}
