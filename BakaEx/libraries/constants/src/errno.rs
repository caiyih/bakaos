#[repr(isize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrNo {
    // Operation not permitted
    OperationNotPermitted = -1,
    // No such file or directory
    NoSuchFileOrDirectory = -2,
    // No such process
    NoSuchProcess = -3,
    // Interrupted system call
    InterruptedSystemCall = -4,
    // Input/output error
    InputOutputError = -5,
    // No such device or address
    NoSuchDeviceOrAddress = -6,
    // Argument list too long
    ArgumentListTooLong = -7,
    // Exec format error
    ExecFormatError = -8,
    // Bad file descriptor
    BadFileDescriptor = -9,
    // No child processes
    NoChildProcesses = -10,
    // Resource temporarily unavailable
    ResourceTemporarilyUnavailable = -11,
    // Cannot allocate memory
    CannotAllocateMemory = -12,
    // Permission denied
    PermissionDenied = -13,
    // Bad address
    BadAddress = -14,
    // Block device required
    BlockDeviceRequired = -15,
    // Device or resource busy
    DeviceOrResourceBusy = -16,
    // File exists
    FileExists = -17,
    // Invalid cross-device link
    InvalidCrossDeviceLink = -18,
    // No such device
    NoSuchDevice = -19,
    // Not a directory
    NotADirectory = -20,
    // Is a directory
    IsADirectory = -21,
    // Invalid argument
    InvalidArgument = -22,
    // Too many open files in system
    TooManyOpenFilesInSystem = -23,
    // Too many open files
    TooManyOpenFiles = -24,
    // Inappropriate ioctl for device
    InappropriateIoctlForDevice = -25,
    // Text file busy
    TextFileBusy = -26,
    // File too large
    FileTooLarge = -27,
    // No space left on device
    NoSpaceLeftOnDevice = -28,
    // Illegal seek
    IllegalSeek = -29,
    // Read-only file system
    ReadOnlyFileSystem = -30,
    // Too many links
    TooManyLinks = -31,
    // Broken pipe
    BrokenPipe = -32,
    // Numerical argument out of domain
    NumericalArgumentOutOfDomain = -33,
    // Numerical result out of range
    NumericalResultOutOfRange = -34,
    // Resource deadlock avoided
    ResourceDeadlockAvoided = -35,
    // File name too long
    FileNameTooLong = -36,
    // No locks available
    NoLocksAvailable = -37,
    // Function not implemented
    FunctionNotImplemented = -38,
    // Directory not empty
    DirectoryNotEmpty = -39,
    // Too many levels of symbolic links
    TooManyLevelsOfSymbolicLinks = -40,
    // Unknown error 41
    UnknownError41 = -41,
    // No message of desired type
    NoMessageOfDesiredType = -42,
    // Identifier removed
    IdentifierRemoved = -43,
    // Channel number out of range
    ChannelNumberOutOfRange = -44,
    // Level 2 not synchronized
    Level2NotSynchronized = -45,
    // Level 3 halted
    Level3Halted = -46,
    // Level 3 reset
    Level3Reset = -47,
    // Link number out of range
    LinkNumberOutOfRange = -48,
    // Protocol driver not attached
    ProtocolDriverNotAttached = -49,
    // No CSI structure available
    NoCsiStructureAvailable = -50,
    // Level 2 halted
    Level2Halted = -51,
    // Invalid exchange
    InvalidExchange = -52,
    // Invalid request descriptor
    InvalidRequestDescriptor = -53,
    // Exchange full
    ExchangeFull = -54,
    // No anode
    NoAnode = -55,
    // Invalid request code
    InvalidRequestCode = -56,
    // Invalid slot
    InvalidSlot = -57,
    // Bad font file format
    BadFontFileFormat = -59,
    // Device not a stream
    DeviceNotAStream = -60,
    // No data available
    NoDataAvailable = -61,
    // Timer expired
    TimerExpired = -62,
    // Out of streams resources
    OutOfStreamsResources = -63,
    // Machine is not on the network
    MachineIsNotOnTheNetwork = -64,
    // Package not installed
    PackageNotInstalled = -65,
    // Object is remote
    ObjectIsRemote = -66,
    // Link has been severed
    LinkHasBeenSevered = -67,
    // Advertise error
    AdvertiseError = -68,
    // Srmount error
    SrmountError = -69,
    // Communication error on send
    CommunicationErrorOnSend = -70,
    // Protocol error
    ProtocolError = -71,
    // Multihop attempted
    MultihopAttempted = -72,
    // RFS specific error
    RfsSpecificError = -73,
    // Bad message
    BadMessage = -74,
    // Value too large for defined data type
    ValueTooLargeForDefinedDataType = -75,
    // Name not unique on network
    NameNotUniqueOnNetwork = -76,
    // File descriptor in bad state
    FileDescriptorInBadState = -77,
    // Remote address changed
    RemoteAddressChanged = -78,
    // Can not access a needed shared library
    CannotAccessANeededSharedLibrary = -79,
    // Accessing a corrupted shared library
    AccessingACorruptedSharedLibrary = -80,
    //.lib section in a.out corrupted
    LibSectionInAOutCorrupted = -81,
    // Attempting to link in too many shared libraries
    AttemptingToLinkInTooManySharedLibraries = -82,
    // Cannot exec a shared library directly
    CannotExecASharedLibraryDirectly = -83,
    // Invalid or incomplete multibyte or wide character
    InvalidOrIncompleteMultibyteOrWideCharacter = -84,
    // Interrupted system call should be restarted
    InterruptedSystemCallShouldBeRestarted = -85,
    // Streams pipe error
    StreamsPipeError = -86,
    // Too many users
    TooManyUsers = -87,
    // Socket operation on non-socket
    SocketOperationOnNonSocket = -88,
    // Destination address required
    DestinationAddressRequired = -89,
    // Message too long
    MessageTooLong = -90,
    // Protocol wrong type for socket
    ProtocolWrongTypeForSocket = -91,
    // Protocol not available
    ProtocolNotAvailable = -92,
    // Protocol not supported
    ProtocolNotSupported = -93,
    // Socket type not supported
    SocketTypeNotSupported = -94,
    // Operation not supported
    OperationNotSupported = -95,
    // Protocol family not supported
    ProtocolFamilyNotSupported = -96,
    // Address family not supported by protocol
    AddressFamilyNotSupportedByProtocol = -97,
    // Address already in use
    AddressAlreadyInUse = -98,
    // Cannot assign requested address
    CannotAssignRequestedAddress = -99,
    // Network is down
    NetworkIsDown = -100,
    // Network is unreachable
    NetworkIsUnreachable = -101,
    // Network dropped connection on reset
    NetworkDroppedConnectionOnReset = -102,
    // Software caused connection abort
    SoftwareCausedConnectionAbort = -103,
    // Connection reset by peer
    ConnectionResetByPeer = -104,
    // No buffer space available
    NoBufferSpaceAvailable = -105,
    // Transport endpoint is already connected
    TransportEndpointIsAlreadyConnected = -106,
    // Transport endpoint is not connected
    TransportEndpointIsNotConnected = -107,
    // Cannot send after transport endpoint shutdown
    CannotSendAfterTransportEndpointShutdown = -108,
    // Too many references: cannot splice
    TooManyReferencesCannotSplice = -109,
    // Connection timed out
    ConnectionTimedOut = -110,
    // Connection refused
    ConnectionRefused = -111,
    // Host is down
    HostIsDown = -112,
    // No route to host
    NoRouteToHost = -113,
    // Operation already in progress
    OperationAlreadyInProgress = -114,
    // Operation now in progress
    OperationNowInProgress = -115,
    // Stale file handle
    StaleFileHandle = -116,
    // Structure needs cleaning
    StructureNeedsCleaning = -117,
    // Not a XENIX named type file
    NotAXenixNamedTypeFile = -118,
    // No XENIX semaphores available
    NoXenixSemaphoresAvailable = -119,
    // Is a named type file
    IsANamedTypeFile = -120,
    // Remote I/O error
    RemoteIOError = -121,
    // Disk quota exceeded
    DiskQuotaExceeded = -122,
    // No medium found
    NoMediumFound = -123,
    // Wrong medium type
    WrongMediumType = -124,
    // Operation canceled
    OperationCanceled = -125,
    // Required key not available
    RequiredKeyNotAvailable = -126,
    // Key has expired
    KeyHasExpired = -127,
    // Key has been revoked
    KeyHasBeenRevoked = -128,
    // Key was rejected by service
    KeyWasRejectedByService = -129,
    // Owner died
    OwnerDied = -130,
    // State not recoverable
    StateNotRecoverable = -131,
    // Operation not possible due to RF-kill
    OperationNotPossibleDueToRfKill = -132,
    // Memory page has hardware error
    MemoryPageHasHardwareError = -133,
}

pub struct SyscallError;

// Result<isize, isize> is the actual type of SyscallResult, so we can return an instance directly
#[allow(non_upper_case_globals)]
impl SyscallError {
    // Success
    pub const Success: Result<isize, isize> = Ok(0);
    // Operation not permitted
    pub const OperationNotPermitted: Result<isize, isize> =
        Err(ErrNo::OperationNotPermitted as isize);
    // No such file or directory
    pub const NoSuchFileOrDirectory: Result<isize, isize> =
        Err(ErrNo::NoSuchFileOrDirectory as isize);
    // No such process
    pub const NoSuchProcess: Result<isize, isize> = Err(ErrNo::NoSuchProcess as isize);
    // Interrupted system call
    pub const InterruptedSystemCall: Result<isize, isize> =
        Err(ErrNo::InterruptedSystemCall as isize);
    // Input/output error
    pub const InputOutputError: Result<isize, isize> = Err(ErrNo::InputOutputError as isize);
    // No such device or address
    pub const NoSuchDeviceOrAddress: Result<isize, isize> =
        Err(ErrNo::NoSuchDeviceOrAddress as isize);
    // Argument list too long
    pub const ArgumentListTooLong: Result<isize, isize> = Err(ErrNo::ArgumentListTooLong as isize);
    // Exec format error
    pub const ExecFormatError: Result<isize, isize> = Err(ErrNo::ExecFormatError as isize);
    // Bad file descriptor
    pub const BadFileDescriptor: Result<isize, isize> = Err(ErrNo::BadFileDescriptor as isize);
    // No child processes
    pub const NoChildProcesses: Result<isize, isize> = Err(ErrNo::NoChildProcesses as isize);
    // Resource temporarily unavailable
    pub const ResourceTemporarilyUnavailable: Result<isize, isize> =
        Err(ErrNo::ResourceTemporarilyUnavailable as isize);
    // Cannot allocate memory
    pub const CannotAllocateMemory: Result<isize, isize> =
        Err(ErrNo::CannotAllocateMemory as isize);
    // Permission denied
    pub const PermissionDenied: Result<isize, isize> = Err(ErrNo::PermissionDenied as isize);
    // Bad address
    pub const BadAddress: Result<isize, isize> = Err(ErrNo::BadAddress as isize);
    // Block device required
    pub const BlockDeviceRequired: Result<isize, isize> = Err(ErrNo::BlockDeviceRequired as isize);
    // Device or resource busy
    pub const DeviceOrResourceBusy: Result<isize, isize> =
        Err(ErrNo::DeviceOrResourceBusy as isize);
    // File exists
    pub const FileExists: Result<isize, isize> = Err(ErrNo::FileExists as isize);
    // Invalid cross-device link
    pub const InvalidCrossDeviceLink: Result<isize, isize> =
        Err(ErrNo::InvalidCrossDeviceLink as isize);
    // No such device
    pub const NoSuchDevice: Result<isize, isize> = Err(ErrNo::NoSuchDevice as isize);
    // Not a directory
    pub const NotADirectory: Result<isize, isize> = Err(ErrNo::NotADirectory as isize);
    // Is a directory
    pub const IsADirectory: Result<isize, isize> = Err(ErrNo::IsADirectory as isize);
    // Invalid argument
    pub const InvalidArgument: Result<isize, isize> = Err(ErrNo::InvalidArgument as isize);
    // Too many open files in system
    pub const TooManyOpenFilesInSystem: Result<isize, isize> =
        Err(ErrNo::TooManyOpenFilesInSystem as isize);
    // Too many open files
    pub const TooManyOpenFiles: Result<isize, isize> = Err(ErrNo::TooManyOpenFiles as isize);
    // Inappropriate ioctl for device
    pub const InappropriateIoctlForDevice: Result<isize, isize> =
        Err(ErrNo::InappropriateIoctlForDevice as isize);
    // Text file busy
    pub const TextFileBusy: Result<isize, isize> = Err(ErrNo::TextFileBusy as isize);
    // File too large
    pub const FileTooLarge: Result<isize, isize> = Err(ErrNo::FileTooLarge as isize);
    // No space left on device
    pub const NoSpaceLeftOnDevice: Result<isize, isize> = Err(ErrNo::NoSpaceLeftOnDevice as isize);
    // Illegal seek
    pub const IllegalSeek: Result<isize, isize> = Err(ErrNo::IllegalSeek as isize);
    // Read-only file system
    pub const ReadOnlyFileSystem: Result<isize, isize> = Err(ErrNo::ReadOnlyFileSystem as isize);
    // Too many links
    pub const TooManyLinks: Result<isize, isize> = Err(ErrNo::TooManyLinks as isize);
    // Broken pipe
    pub const BrokenPipe: Result<isize, isize> = Err(ErrNo::BrokenPipe as isize);
    // Numerical argument out of domain
    pub const NumericalArgumentOutOfDomain: Result<isize, isize> =
        Err(ErrNo::NumericalArgumentOutOfDomain as isize);
    // Numerical result out of range
    pub const NumericalResultOutOfRange: Result<isize, isize> =
        Err(ErrNo::NumericalResultOutOfRange as isize);
    // Resource deadlock avoided
    pub const ResourceDeadlockAvoided: Result<isize, isize> =
        Err(ErrNo::ResourceDeadlockAvoided as isize);
    // File name too long
    pub const FileNameTooLong: Result<isize, isize> = Err(ErrNo::FileNameTooLong as isize);
    // No locks available
    pub const NoLocksAvailable: Result<isize, isize> = Err(ErrNo::NoLocksAvailable as isize);
    // Function not implemented
    pub const FunctionNotImplemented: Result<isize, isize> =
        Err(ErrNo::FunctionNotImplemented as isize);
    // Directory not empty
    pub const DirectoryNotEmpty: Result<isize, isize> = Err(ErrNo::DirectoryNotEmpty as isize);
    // Too many levels of symbolic links
    pub const TooManyLevelsOfSymbolicLinks: Result<isize, isize> =
        Err(ErrNo::TooManyLevelsOfSymbolicLinks as isize);
    // Unknown error 41
    pub const UnknownError41: Result<isize, isize> = Err(ErrNo::UnknownError41 as isize);
    // No message of desired type
    pub const NoMessageOfDesiredType: Result<isize, isize> =
        Err(ErrNo::NoMessageOfDesiredType as isize);
    // Identifier removed
    pub const IdentifierRemoved: Result<isize, isize> = Err(ErrNo::IdentifierRemoved as isize);
    // Channel number out of range
    pub const ChannelNumberOutOfRange: Result<isize, isize> =
        Err(ErrNo::ChannelNumberOutOfRange as isize);
    // Level 2 not synchronized
    pub const Level2NotSynchronized: Result<isize, isize> =
        Err(ErrNo::Level2NotSynchronized as isize);
    // Level 3 halted
    pub const Level3Halted: Result<isize, isize> = Err(ErrNo::Level3Halted as isize);
    // Level 3 reset
    pub const Level3Reset: Result<isize, isize> = Err(ErrNo::Level3Reset as isize);
    // Link number out of range
    pub const LinkNumberOutOfRange: Result<isize, isize> =
        Err(ErrNo::LinkNumberOutOfRange as isize);
    // Protocol driver not attached
    pub const ProtocolDriverNotAttached: Result<isize, isize> =
        Err(ErrNo::ProtocolDriverNotAttached as isize);
    // No CSI structure available
    pub const NoCsiStructureAvailable: Result<isize, isize> =
        Err(ErrNo::NoCsiStructureAvailable as isize);
    // Level 2 halted
    pub const Level2Halted: Result<isize, isize> = Err(ErrNo::Level2Halted as isize);
    // Invalid exchange
    pub const InvalidExchange: Result<isize, isize> = Err(ErrNo::InvalidExchange as isize);
    // Invalid request descriptor
    pub const InvalidRequestDescriptor: Result<isize, isize> =
        Err(ErrNo::InvalidRequestDescriptor as isize);
    // Exchange full
    pub const ExchangeFull: Result<isize, isize> = Err(ErrNo::ExchangeFull as isize);
    // No anode
    pub const NoAnode: Result<isize, isize> = Err(ErrNo::NoAnode as isize);
    // Invalid request code
    pub const InvalidRequestCode: Result<isize, isize> = Err(ErrNo::InvalidRequestCode as isize);
    // Invalid slot
    pub const InvalidSlot: Result<isize, isize> = Err(ErrNo::InvalidSlot as isize);
    // Bad font file format
    pub const BadFontFileFormat: Result<isize, isize> = Err(ErrNo::BadFontFileFormat as isize);
    // Device not a stream
    pub const DeviceNotAStream: Result<isize, isize> = Err(ErrNo::DeviceNotAStream as isize);
    // No data available
    pub const NoDataAvailable: Result<isize, isize> = Err(ErrNo::NoDataAvailable as isize);
    // Timer expired
    pub const TimerExpired: Result<isize, isize> = Err(ErrNo::TimerExpired as isize);
    // Out of streams resources
    pub const OutOfStreamsResources: Result<isize, isize> =
        Err(ErrNo::OutOfStreamsResources as isize);
    // Machine is not on the network
    pub const MachineIsNotOnTheNetwork: Result<isize, isize> =
        Err(ErrNo::MachineIsNotOnTheNetwork as isize);
    // Package not installed
    pub const PackageNotInstalled: Result<isize, isize> = Err(ErrNo::PackageNotInstalled as isize);
    // Object is remote
    pub const ObjectIsRemote: Result<isize, isize> = Err(ErrNo::ObjectIsRemote as isize);
    // Link has been severed
    pub const LinkHasBeenSevered: Result<isize, isize> = Err(ErrNo::LinkHasBeenSevered as isize);
    // Advertise error
    pub const AdvertiseError: Result<isize, isize> = Err(ErrNo::AdvertiseError as isize);
    // Srmount error
    pub const SrmountError: Result<isize, isize> = Err(ErrNo::SrmountError as isize);
    // Communication error on send
    pub const CommunicationErrorOnSend: Result<isize, isize> =
        Err(ErrNo::CommunicationErrorOnSend as isize);
    // Protocol error
    pub const ProtocolError: Result<isize, isize> = Err(ErrNo::ProtocolError as isize);
    // Multihop attempted
    pub const MultihopAttempted: Result<isize, isize> = Err(ErrNo::MultihopAttempted as isize);
    // RFS specific error
    pub const RfsSpecificError: Result<isize, isize> = Err(ErrNo::RfsSpecificError as isize);
    // Bad message
    pub const BadMessage: Result<isize, isize> = Err(ErrNo::BadMessage as isize);
    // Value too large for defined data type
    pub const ValueTooLargeForDefinedDataType: Result<isize, isize> =
        Err(ErrNo::ValueTooLargeForDefinedDataType as isize);
    // Name not unique on network
    pub const NameNotUniqueOnNetwork: Result<isize, isize> =
        Err(ErrNo::NameNotUniqueOnNetwork as isize);
    // File descriptor in bad state
    pub const FileDescriptorInBadState: Result<isize, isize> =
        Err(ErrNo::FileDescriptorInBadState as isize);
    // Remote address changed
    pub const RemoteAddressChanged: Result<isize, isize> =
        Err(ErrNo::RemoteAddressChanged as isize);
    // Can not access a needed shared library
    pub const CannotAccessANeededSharedLibrary: Result<isize, isize> =
        Err(ErrNo::CannotAccessANeededSharedLibrary as isize);
    // Accessing a corrupted shared library
    pub const AccessingACorruptedSharedLibrary: Result<isize, isize> =
        Err(ErrNo::AccessingACorruptedSharedLibrary as isize);
    //.lib section in a.out corrupted
    pub const LibSectionInAOutCorrupted: Result<isize, isize> =
        Err(ErrNo::LibSectionInAOutCorrupted as isize);
    // Attempting to link in too many shared libraries
    pub const AttemptingToLinkInTooManySharedLibraries: Result<isize, isize> =
        Err(ErrNo::AttemptingToLinkInTooManySharedLibraries as isize);
    // Cannot exec a shared library directly
    pub const CannotExecASharedLibraryDirectly: Result<isize, isize> =
        Err(ErrNo::CannotExecASharedLibraryDirectly as isize);
    // Invalid or incomplete multibyte or wide character
    pub const InvalidOrIncompleteMultibyteOrWideCharacter: Result<isize, isize> =
        Err(ErrNo::InvalidOrIncompleteMultibyteOrWideCharacter as isize);
    // Interrupted system call should be restarted
    pub const InterruptedSystemCallShouldBeRestarted: Result<isize, isize> =
        Err(ErrNo::InterruptedSystemCallShouldBeRestarted as isize);
    // Streams pipe error
    pub const StreamsPipeError: Result<isize, isize> = Err(ErrNo::StreamsPipeError as isize);
    // Too many users
    pub const TooManyUsers: Result<isize, isize> = Err(ErrNo::TooManyUsers as isize);
    // Socket operation on non-socket
    pub const SocketOperationOnNonSocket: Result<isize, isize> =
        Err(ErrNo::SocketOperationOnNonSocket as isize);
    // Destination address required
    pub const DestinationAddressRequired: Result<isize, isize> =
        Err(ErrNo::DestinationAddressRequired as isize);
    // Message too long
    pub const MessageTooLong: Result<isize, isize> = Err(ErrNo::MessageTooLong as isize);
    // Protocol wrong type for socket
    pub const ProtocolWrongTypeForSocket: Result<isize, isize> =
        Err(ErrNo::ProtocolWrongTypeForSocket as isize);
    // Protocol not available
    pub const ProtocolNotAvailable: Result<isize, isize> =
        Err(ErrNo::ProtocolNotAvailable as isize);
    // Protocol not supported
    pub const ProtocolNotSupported: Result<isize, isize> =
        Err(ErrNo::ProtocolNotSupported as isize);
    // Socket type not supported
    pub const SocketTypeNotSupported: Result<isize, isize> =
        Err(ErrNo::SocketTypeNotSupported as isize);
    // Operation not supported
    pub const OperationNotSupported: Result<isize, isize> =
        Err(ErrNo::OperationNotSupported as isize);
    // Protocol family not supported
    pub const ProtocolFamilyNotSupported: Result<isize, isize> =
        Err(ErrNo::ProtocolFamilyNotSupported as isize);
    // Address family not supported by protocol
    pub const AddressFamilyNotSupportedByProtocol: Result<isize, isize> =
        Err(ErrNo::AddressFamilyNotSupportedByProtocol as isize);
    // Address already in use
    pub const AddressAlreadyInUse: Result<isize, isize> = Err(ErrNo::AddressAlreadyInUse as isize);
    // Cannot assign requested address
    pub const CannotAssignRequestedAddress: Result<isize, isize> =
        Err(ErrNo::CannotAssignRequestedAddress as isize);
    // Network is down
    pub const NetworkIsDown: Result<isize, isize> = Err(ErrNo::NetworkIsDown as isize);
    // Network is unreachable
    pub const NetworkIsUnreachable: Result<isize, isize> =
        Err(ErrNo::NetworkIsUnreachable as isize);
    // Network dropped connection on reset
    pub const NetworkDroppedConnectionOnReset: Result<isize, isize> =
        Err(ErrNo::NetworkDroppedConnectionOnReset as isize);
    // Software caused connection abort
    pub const SoftwareCausedConnectionAbort: Result<isize, isize> =
        Err(ErrNo::SoftwareCausedConnectionAbort as isize);
    // Connection reset by peer
    pub const ConnectionResetByPeer: Result<isize, isize> =
        Err(ErrNo::ConnectionResetByPeer as isize);
    // No buffer space available
    pub const NoBufferSpaceAvailable: Result<isize, isize> =
        Err(ErrNo::NoBufferSpaceAvailable as isize);
    // Transport endpoint is already connected
    pub const TransportEndpointIsAlreadyConnected: Result<isize, isize> =
        Err(ErrNo::TransportEndpointIsAlreadyConnected as isize);
    // Transport endpoint is not connected
    pub const TransportEndpointIsNotConnected: Result<isize, isize> =
        Err(ErrNo::TransportEndpointIsNotConnected as isize);
    // Cannot send after transport endpoint shutdown
    pub const CannotSendAfterTransportEndpointShutdown: Result<isize, isize> =
        Err(ErrNo::CannotSendAfterTransportEndpointShutdown as isize);
    // Too many references: cannot splice
    pub const TooManyReferencesCannotSplice: Result<isize, isize> =
        Err(ErrNo::TooManyReferencesCannotSplice as isize);
    // Connection timed out
    pub const ConnectionTimedOut: Result<isize, isize> = Err(ErrNo::ConnectionTimedOut as isize);
    // Connection refused
    pub const ConnectionRefused: Result<isize, isize> = Err(ErrNo::ConnectionRefused as isize);
    // Host is down
    pub const HostIsDown: Result<isize, isize> = Err(ErrNo::HostIsDown as isize);
    // No route to host
    pub const NoRouteToHost: Result<isize, isize> = Err(ErrNo::NoRouteToHost as isize);
    // Operation already in progress
    pub const OperationAlreadyInProgress: Result<isize, isize> =
        Err(ErrNo::OperationAlreadyInProgress as isize);
    // Operation now in progress
    pub const OperationNowInProgress: Result<isize, isize> =
        Err(ErrNo::OperationNowInProgress as isize);
    // Stale file handle
    pub const StaleFileHandle: Result<isize, isize> = Err(ErrNo::StaleFileHandle as isize);
    // Structure needs cleaning
    pub const StructureNeedsCleaning: Result<isize, isize> =
        Err(ErrNo::StructureNeedsCleaning as isize);
    // Not a XENIX named type file
    pub const NotAXenixNamedTypeFile: Result<isize, isize> =
        Err(ErrNo::NotAXenixNamedTypeFile as isize);
    // No XENIX semaphores available
    pub const NoXenixSemaphoresAvailable: Result<isize, isize> =
        Err(ErrNo::NoXenixSemaphoresAvailable as isize);
    // Is a named type file
    pub const IsANamedTypeFile: Result<isize, isize> = Err(ErrNo::IsANamedTypeFile as isize);
    // Remote I/O error
    pub const RemoteIOError: Result<isize, isize> = Err(ErrNo::RemoteIOError as isize);
    // Disk quota exceeded
    pub const DiskQuotaExceeded: Result<isize, isize> = Err(ErrNo::DiskQuotaExceeded as isize);
    // No medium found
    pub const NoMediumFound: Result<isize, isize> = Err(ErrNo::NoMediumFound as isize);
    // Wrong medium type
    pub const WrongMediumType: Result<isize, isize> = Err(ErrNo::WrongMediumType as isize);
    // Operation canceled
    pub const OperationCanceled: Result<isize, isize> = Err(ErrNo::OperationCanceled as isize);
    // Required key not available
    pub const RequiredKeyNotAvailable: Result<isize, isize> =
        Err(ErrNo::RequiredKeyNotAvailable as isize);
    // Key has expired
    pub const KeyHasExpired: Result<isize, isize> = Err(ErrNo::KeyHasExpired as isize);
    // Key has been revoked
    pub const KeyHasBeenRevoked: Result<isize, isize> = Err(ErrNo::KeyHasBeenRevoked as isize);
    // Key was rejected by service
    pub const KeyWasRejectedByService: Result<isize, isize> =
        Err(ErrNo::KeyWasRejectedByService as isize);
    // Owner died
    pub const OwnerDied: Result<isize, isize> = Err(ErrNo::OwnerDied as isize);
    // State not recoverable
    pub const StateNotRecoverable: Result<isize, isize> = Err(ErrNo::StateNotRecoverable as isize);
    // Operation not possible due to RF-kill
    pub const OperationNotPossibleDueToRfKill: Result<isize, isize> =
        Err(ErrNo::OperationNotPossibleDueToRfKill as isize);
    // Memory page has hardware error
    pub const MemoryPageHasHardwareError: Result<isize, isize> =
        Err(ErrNo::MemoryPageHasHardwareError as isize);
}
