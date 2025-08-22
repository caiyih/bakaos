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

// Result<isize, ErrNo> is the actual type of SyscallResult, so we can return an instance directly
#[allow(non_upper_case_globals)]
impl SyscallError {
    // Success
    pub const Success: Result<isize, ErrNo> = Ok(0);
    // Operation not permitted
    pub const OperationNotPermitted: Result<isize, ErrNo> = Err(ErrNo::OperationNotPermitted);
    // No such file or directory
    pub const NoSuchFileOrDirectory: Result<isize, ErrNo> = Err(ErrNo::NoSuchFileOrDirectory);
    // No such process
    pub const NoSuchProcess: Result<isize, ErrNo> = Err(ErrNo::NoSuchProcess);
    // Interrupted system call
    pub const InterruptedSystemCall: Result<isize, ErrNo> = Err(ErrNo::InterruptedSystemCall);
    // Input/output error
    pub const InputOutputError: Result<isize, ErrNo> = Err(ErrNo::InputOutputError);
    // No such device or address
    pub const NoSuchDeviceOrAddress: Result<isize, ErrNo> = Err(ErrNo::NoSuchDeviceOrAddress);
    // Argument list too long
    pub const ArgumentListTooLong: Result<isize, ErrNo> = Err(ErrNo::ArgumentListTooLong);
    // Exec format error
    pub const ExecFormatError: Result<isize, ErrNo> = Err(ErrNo::ExecFormatError);
    // Bad file descriptor
    pub const BadFileDescriptor: Result<isize, ErrNo> = Err(ErrNo::BadFileDescriptor);
    // No child processes
    pub const NoChildProcesses: Result<isize, ErrNo> = Err(ErrNo::NoChildProcesses);
    // Resource temporarily unavailable
    pub const ResourceTemporarilyUnavailable: Result<isize, ErrNo> =
        Err(ErrNo::ResourceTemporarilyUnavailable);
    // Cannot allocate memory
    pub const CannotAllocateMemory: Result<isize, ErrNo> = Err(ErrNo::CannotAllocateMemory);
    // Permission denied
    pub const PermissionDenied: Result<isize, ErrNo> = Err(ErrNo::PermissionDenied);
    // Bad address
    pub const BadAddress: Result<isize, ErrNo> = Err(ErrNo::BadAddress);
    // Block device required
    pub const BlockDeviceRequired: Result<isize, ErrNo> = Err(ErrNo::BlockDeviceRequired);
    // Device or resource busy
    pub const DeviceOrResourceBusy: Result<isize, ErrNo> = Err(ErrNo::DeviceOrResourceBusy);
    // File exists
    pub const FileExists: Result<isize, ErrNo> = Err(ErrNo::FileExists);
    // Invalid cross-device link
    pub const InvalidCrossDeviceLink: Result<isize, ErrNo> = Err(ErrNo::InvalidCrossDeviceLink);
    // No such device
    pub const NoSuchDevice: Result<isize, ErrNo> = Err(ErrNo::NoSuchDevice);
    // Not a directory
    pub const NotADirectory: Result<isize, ErrNo> = Err(ErrNo::NotADirectory);
    // Is a directory
    pub const IsADirectory: Result<isize, ErrNo> = Err(ErrNo::IsADirectory);
    // Invalid argument
    pub const InvalidArgument: Result<isize, ErrNo> = Err(ErrNo::InvalidArgument);
    // Too many open files in system
    pub const TooManyOpenFilesInSystem: Result<isize, ErrNo> = Err(ErrNo::TooManyOpenFilesInSystem);
    // Too many open files
    pub const TooManyOpenFiles: Result<isize, ErrNo> = Err(ErrNo::TooManyOpenFiles);
    // Inappropriate ioctl for device
    pub const InappropriateIoctlForDevice: Result<isize, ErrNo> =
        Err(ErrNo::InappropriateIoctlForDevice);
    // Text file busy
    pub const TextFileBusy: Result<isize, ErrNo> = Err(ErrNo::TextFileBusy);
    // File too large
    pub const FileTooLarge: Result<isize, ErrNo> = Err(ErrNo::FileTooLarge);
    // No space left on device
    pub const NoSpaceLeftOnDevice: Result<isize, ErrNo> = Err(ErrNo::NoSpaceLeftOnDevice);
    // Illegal seek
    pub const IllegalSeek: Result<isize, ErrNo> = Err(ErrNo::IllegalSeek);
    // Read-only file system
    pub const ReadOnlyFileSystem: Result<isize, ErrNo> = Err(ErrNo::ReadOnlyFileSystem);
    // Too many links
    pub const TooManyLinks: Result<isize, ErrNo> = Err(ErrNo::TooManyLinks);
    // Broken pipe
    pub const BrokenPipe: Result<isize, ErrNo> = Err(ErrNo::BrokenPipe);
    // Numerical argument out of domain
    pub const NumericalArgumentOutOfDomain: Result<isize, ErrNo> =
        Err(ErrNo::NumericalArgumentOutOfDomain);
    // Numerical result out of range
    pub const NumericalResultOutOfRange: Result<isize, ErrNo> =
        Err(ErrNo::NumericalResultOutOfRange);
    // Resource deadlock avoided
    pub const ResourceDeadlockAvoided: Result<isize, ErrNo> = Err(ErrNo::ResourceDeadlockAvoided);
    // File name too long
    pub const FileNameTooLong: Result<isize, ErrNo> = Err(ErrNo::FileNameTooLong);
    // No locks available
    pub const NoLocksAvailable: Result<isize, ErrNo> = Err(ErrNo::NoLocksAvailable);
    // Function not implemented
    pub const FunctionNotImplemented: Result<isize, ErrNo> = Err(ErrNo::FunctionNotImplemented);
    // Directory not empty
    pub const DirectoryNotEmpty: Result<isize, ErrNo> = Err(ErrNo::DirectoryNotEmpty);
    // Too many levels of symbolic links
    pub const TooManyLevelsOfSymbolicLinks: Result<isize, ErrNo> =
        Err(ErrNo::TooManyLevelsOfSymbolicLinks);
    // Unknown error 41
    pub const UnknownError41: Result<isize, ErrNo> = Err(ErrNo::UnknownError41);
    // No message of desired type
    pub const NoMessageOfDesiredType: Result<isize, ErrNo> = Err(ErrNo::NoMessageOfDesiredType);
    // Identifier removed
    pub const IdentifierRemoved: Result<isize, ErrNo> = Err(ErrNo::IdentifierRemoved);
    // Channel number out of range
    pub const ChannelNumberOutOfRange: Result<isize, ErrNo> = Err(ErrNo::ChannelNumberOutOfRange);
    // Level 2 not synchronized
    pub const Level2NotSynchronized: Result<isize, ErrNo> = Err(ErrNo::Level2NotSynchronized);
    // Level 3 halted
    pub const Level3Halted: Result<isize, ErrNo> = Err(ErrNo::Level3Halted);
    // Level 3 reset
    pub const Level3Reset: Result<isize, ErrNo> = Err(ErrNo::Level3Reset);
    // Link number out of range
    pub const LinkNumberOutOfRange: Result<isize, ErrNo> = Err(ErrNo::LinkNumberOutOfRange);
    // Protocol driver not attached
    pub const ProtocolDriverNotAttached: Result<isize, ErrNo> =
        Err(ErrNo::ProtocolDriverNotAttached);
    // No CSI structure available
    pub const NoCsiStructureAvailable: Result<isize, ErrNo> = Err(ErrNo::NoCsiStructureAvailable);
    // Level 2 halted
    pub const Level2Halted: Result<isize, ErrNo> = Err(ErrNo::Level2Halted);
    // Invalid exchange
    pub const InvalidExchange: Result<isize, ErrNo> = Err(ErrNo::InvalidExchange);
    // Invalid request descriptor
    pub const InvalidRequestDescriptor: Result<isize, ErrNo> = Err(ErrNo::InvalidRequestDescriptor);
    // Exchange full
    pub const ExchangeFull: Result<isize, ErrNo> = Err(ErrNo::ExchangeFull);
    // No anode
    pub const NoAnode: Result<isize, ErrNo> = Err(ErrNo::NoAnode);
    // Invalid request code
    pub const InvalidRequestCode: Result<isize, ErrNo> = Err(ErrNo::InvalidRequestCode);
    // Invalid slot
    pub const InvalidSlot: Result<isize, ErrNo> = Err(ErrNo::InvalidSlot);
    // Bad font file format
    pub const BadFontFileFormat: Result<isize, ErrNo> = Err(ErrNo::BadFontFileFormat);
    // Device not a stream
    pub const DeviceNotAStream: Result<isize, ErrNo> = Err(ErrNo::DeviceNotAStream);
    // No data available
    pub const NoDataAvailable: Result<isize, ErrNo> = Err(ErrNo::NoDataAvailable);
    // Timer expired
    pub const TimerExpired: Result<isize, ErrNo> = Err(ErrNo::TimerExpired);
    // Out of streams resources
    pub const OutOfStreamsResources: Result<isize, ErrNo> = Err(ErrNo::OutOfStreamsResources);
    // Machine is not on the network
    pub const MachineIsNotOnTheNetwork: Result<isize, ErrNo> = Err(ErrNo::MachineIsNotOnTheNetwork);
    // Package not installed
    pub const PackageNotInstalled: Result<isize, ErrNo> = Err(ErrNo::PackageNotInstalled);
    // Object is remote
    pub const ObjectIsRemote: Result<isize, ErrNo> = Err(ErrNo::ObjectIsRemote);
    // Link has been severed
    pub const LinkHasBeenSevered: Result<isize, ErrNo> = Err(ErrNo::LinkHasBeenSevered);
    // Advertise error
    pub const AdvertiseError: Result<isize, ErrNo> = Err(ErrNo::AdvertiseError);
    // Srmount error
    pub const SrmountError: Result<isize, ErrNo> = Err(ErrNo::SrmountError);
    // Communication error on send
    pub const CommunicationErrorOnSend: Result<isize, ErrNo> = Err(ErrNo::CommunicationErrorOnSend);
    // Protocol error
    pub const ProtocolError: Result<isize, ErrNo> = Err(ErrNo::ProtocolError);
    // Multihop attempted
    pub const MultihopAttempted: Result<isize, ErrNo> = Err(ErrNo::MultihopAttempted);
    // RFS specific error
    pub const RfsSpecificError: Result<isize, ErrNo> = Err(ErrNo::RfsSpecificError);
    // Bad message
    pub const BadMessage: Result<isize, ErrNo> = Err(ErrNo::BadMessage);
    // Value too large for defined data type
    pub const ValueTooLargeForDefinedDataType: Result<isize, ErrNo> =
        Err(ErrNo::ValueTooLargeForDefinedDataType);
    // Name not unique on network
    pub const NameNotUniqueOnNetwork: Result<isize, ErrNo> = Err(ErrNo::NameNotUniqueOnNetwork);
    // File descriptor in bad state
    pub const FileDescriptorInBadState: Result<isize, ErrNo> = Err(ErrNo::FileDescriptorInBadState);
    // Remote address changed
    pub const RemoteAddressChanged: Result<isize, ErrNo> = Err(ErrNo::RemoteAddressChanged);
    // Can not access a needed shared library
    pub const CannotAccessANeededSharedLibrary: Result<isize, ErrNo> =
        Err(ErrNo::CannotAccessANeededSharedLibrary);
    // Accessing a corrupted shared library
    pub const AccessingACorruptedSharedLibrary: Result<isize, ErrNo> =
        Err(ErrNo::AccessingACorruptedSharedLibrary);
    //.lib section in a.out corrupted
    pub const LibSectionInAOutCorrupted: Result<isize, ErrNo> =
        Err(ErrNo::LibSectionInAOutCorrupted);
    // Attempting to link in too many shared libraries
    pub const AttemptingToLinkInTooManySharedLibraries: Result<isize, ErrNo> =
        Err(ErrNo::AttemptingToLinkInTooManySharedLibraries);
    // Cannot exec a shared library directly
    pub const CannotExecASharedLibraryDirectly: Result<isize, ErrNo> =
        Err(ErrNo::CannotExecASharedLibraryDirectly);
    // Invalid or incomplete multibyte or wide character
    pub const InvalidOrIncompleteMultibyteOrWideCharacter: Result<isize, ErrNo> =
        Err(ErrNo::InvalidOrIncompleteMultibyteOrWideCharacter);
    // Interrupted system call should be restarted
    pub const InterruptedSystemCallShouldBeRestarted: Result<isize, ErrNo> =
        Err(ErrNo::InterruptedSystemCallShouldBeRestarted);
    // Streams pipe error
    pub const StreamsPipeError: Result<isize, ErrNo> = Err(ErrNo::StreamsPipeError);
    // Too many users
    pub const TooManyUsers: Result<isize, ErrNo> = Err(ErrNo::TooManyUsers);
    // Socket operation on non-socket
    pub const SocketOperationOnNonSocket: Result<isize, ErrNo> =
        Err(ErrNo::SocketOperationOnNonSocket);
    // Destination address required
    pub const DestinationAddressRequired: Result<isize, ErrNo> =
        Err(ErrNo::DestinationAddressRequired);
    // Message too long
    pub const MessageTooLong: Result<isize, ErrNo> = Err(ErrNo::MessageTooLong);
    // Protocol wrong type for socket
    pub const ProtocolWrongTypeForSocket: Result<isize, ErrNo> =
        Err(ErrNo::ProtocolWrongTypeForSocket);
    // Protocol not available
    pub const ProtocolNotAvailable: Result<isize, ErrNo> = Err(ErrNo::ProtocolNotAvailable);
    // Protocol not supported
    pub const ProtocolNotSupported: Result<isize, ErrNo> = Err(ErrNo::ProtocolNotSupported);
    // Socket type not supported
    pub const SocketTypeNotSupported: Result<isize, ErrNo> = Err(ErrNo::SocketTypeNotSupported);
    // Operation not supported
    pub const OperationNotSupported: Result<isize, ErrNo> = Err(ErrNo::OperationNotSupported);
    // Protocol family not supported
    pub const ProtocolFamilyNotSupported: Result<isize, ErrNo> =
        Err(ErrNo::ProtocolFamilyNotSupported);
    // Address family not supported by protocol
    pub const AddressFamilyNotSupportedByProtocol: Result<isize, ErrNo> =
        Err(ErrNo::AddressFamilyNotSupportedByProtocol);
    // Address already in use
    pub const AddressAlreadyInUse: Result<isize, ErrNo> = Err(ErrNo::AddressAlreadyInUse);
    // Cannot assign requested address
    pub const CannotAssignRequestedAddress: Result<isize, ErrNo> =
        Err(ErrNo::CannotAssignRequestedAddress);
    // Network is down
    pub const NetworkIsDown: Result<isize, ErrNo> = Err(ErrNo::NetworkIsDown);
    // Network is unreachable
    pub const NetworkIsUnreachable: Result<isize, ErrNo> = Err(ErrNo::NetworkIsUnreachable);
    // Network dropped connection on reset
    pub const NetworkDroppedConnectionOnReset: Result<isize, ErrNo> =
        Err(ErrNo::NetworkDroppedConnectionOnReset);
    // Software caused connection abort
    pub const SoftwareCausedConnectionAbort: Result<isize, ErrNo> =
        Err(ErrNo::SoftwareCausedConnectionAbort);
    // Connection reset by peer
    pub const ConnectionResetByPeer: Result<isize, ErrNo> = Err(ErrNo::ConnectionResetByPeer);
    // No buffer space available
    pub const NoBufferSpaceAvailable: Result<isize, ErrNo> = Err(ErrNo::NoBufferSpaceAvailable);
    // Transport endpoint is already connected
    pub const TransportEndpointIsAlreadyConnected: Result<isize, ErrNo> =
        Err(ErrNo::TransportEndpointIsAlreadyConnected);
    // Transport endpoint is not connected
    pub const TransportEndpointIsNotConnected: Result<isize, ErrNo> =
        Err(ErrNo::TransportEndpointIsNotConnected);
    // Cannot send after transport endpoint shutdown
    pub const CannotSendAfterTransportEndpointShutdown: Result<isize, ErrNo> =
        Err(ErrNo::CannotSendAfterTransportEndpointShutdown);
    // Too many references: cannot splice
    pub const TooManyReferencesCannotSplice: Result<isize, ErrNo> =
        Err(ErrNo::TooManyReferencesCannotSplice);
    // Connection timed out
    pub const ConnectionTimedOut: Result<isize, ErrNo> = Err(ErrNo::ConnectionTimedOut);
    // Connection refused
    pub const ConnectionRefused: Result<isize, ErrNo> = Err(ErrNo::ConnectionRefused);
    // Host is down
    pub const HostIsDown: Result<isize, ErrNo> = Err(ErrNo::HostIsDown);
    // No route to host
    pub const NoRouteToHost: Result<isize, ErrNo> = Err(ErrNo::NoRouteToHost);
    // Operation already in progress
    pub const OperationAlreadyInProgress: Result<isize, ErrNo> =
        Err(ErrNo::OperationAlreadyInProgress);
    // Operation now in progress
    pub const OperationNowInProgress: Result<isize, ErrNo> = Err(ErrNo::OperationNowInProgress);
    // Stale file handle
    pub const StaleFileHandle: Result<isize, ErrNo> = Err(ErrNo::StaleFileHandle);
    // Structure needs cleaning
    pub const StructureNeedsCleaning: Result<isize, ErrNo> = Err(ErrNo::StructureNeedsCleaning);
    // Not a XENIX named type file
    pub const NotAXenixNamedTypeFile: Result<isize, ErrNo> = Err(ErrNo::NotAXenixNamedTypeFile);
    // No XENIX semaphores available
    pub const NoXenixSemaphoresAvailable: Result<isize, ErrNo> =
        Err(ErrNo::NoXenixSemaphoresAvailable);
    // Is a named type file
    pub const IsANamedTypeFile: Result<isize, ErrNo> = Err(ErrNo::IsANamedTypeFile);
    // Remote I/O error
    pub const RemoteIOError: Result<isize, ErrNo> = Err(ErrNo::RemoteIOError);
    // Disk quota exceeded
    pub const DiskQuotaExceeded: Result<isize, ErrNo> = Err(ErrNo::DiskQuotaExceeded);
    // No medium found
    pub const NoMediumFound: Result<isize, ErrNo> = Err(ErrNo::NoMediumFound);
    // Wrong medium type
    pub const WrongMediumType: Result<isize, ErrNo> = Err(ErrNo::WrongMediumType);
    // Operation canceled
    pub const OperationCanceled: Result<isize, ErrNo> = Err(ErrNo::OperationCanceled);
    // Required key not available
    pub const RequiredKeyNotAvailable: Result<isize, ErrNo> = Err(ErrNo::RequiredKeyNotAvailable);
    // Key has expired
    pub const KeyHasExpired: Result<isize, ErrNo> = Err(ErrNo::KeyHasExpired);
    // Key has been revoked
    pub const KeyHasBeenRevoked: Result<isize, ErrNo> = Err(ErrNo::KeyHasBeenRevoked);
    // Key was rejected by service
    pub const KeyWasRejectedByService: Result<isize, ErrNo> = Err(ErrNo::KeyWasRejectedByService);
    // Owner died
    pub const OwnerDied: Result<isize, ErrNo> = Err(ErrNo::OwnerDied);
    // State not recoverable
    pub const StateNotRecoverable: Result<isize, ErrNo> = Err(ErrNo::StateNotRecoverable);
    // Operation not possible due to RF-kill
    pub const OperationNotPossibleDueToRfKill: Result<isize, ErrNo> =
        Err(ErrNo::OperationNotPossibleDueToRfKill);
    // Memory page has hardware error
    pub const MemoryPageHasHardwareError: Result<isize, ErrNo> =
        Err(ErrNo::MemoryPageHasHardwareError);
}
