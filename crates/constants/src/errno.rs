pub struct ErrNo;

#[allow(non_upper_case_globals)]
impl ErrNo {
    // Success
    pub const Success: isize = 0;
    // Operation not permitted
    pub const OperationNotPermitted: isize = -1;
    // No such file or directory
    pub const NoSuchFileOrDirectory: isize = -2;
    // No such process
    pub const NoSuchProcess: isize = -3;
    // Interrupted system call
    pub const InterruptedSystemCall: isize = -4;
    // Input/output error
    pub const InputOutputError: isize = -5;
    // No such device or address
    pub const NoSuchDeviceOrAddress: isize = -6;
    // Argument list too long
    pub const ArgumentListTooLong: isize = -7;
    // Exec format error
    pub const ExecFormatError: isize = -8;
    // Bad file descriptor
    pub const BadFileDescriptor: isize = -9;
    // No child processes
    pub const NoChildProcesses: isize = -10;
    // Resource temporarily unavailable
    pub const ResourceTemporarilyUnavailable: isize = -11;
    // Cannot allocate memory
    pub const CannotAllocateMemory: isize = -12;
    // Permission denied
    pub const PermissionDenied: isize = -13;
    // Bad address
    pub const BadAddress: isize = -14;
    // Block device required
    pub const BlockDeviceRequired: isize = -15;
    // Device or resource busy
    pub const DeviceOrResourceBusy: isize = -16;
    // File exists
    pub const FileExists: isize = -17;
    // Invalid cross-device link
    pub const InvalidCrossDeviceLink: isize = -18;
    // No such device
    pub const NoSuchDevice: isize = -19;
    // Not a directory
    pub const NotADirectory: isize = -20;
    // Is a directory
    pub const IsADirectory: isize = -21;
    // Invalid argument
    pub const InvalidArgument: isize = -22;
    // Too many open files in system
    pub const TooManyOpenFilesInSystem: isize = -23;
    // Too many open files
    pub const TooManyOpenFiles: isize = -24;
    // Inappropriate ioctl for device
    pub const InappropriateIoctlForDevice: isize = -25;
    // Text file busy
    pub const TextFileBusy: isize = -26;
    // File too large
    pub const FileTooLarge: isize = -27;
    // No space left on device
    pub const NoSpaceLeftOnDevice: isize = -28;
    // Illegal seek
    pub const IllegalSeek: isize = -29;
    // Read-only file system
    pub const ReadOnlyFileSystem: isize = -30;
    // Too many links
    pub const TooManyLinks: isize = -31;
    // Broken pipe
    pub const BrokenPipe: isize = -32;
    // Numerical argument out of domain
    pub const NumericalArgumentOutOfDomain: isize = -33;
    // Numerical result out of range
    pub const NumericalResultOutOfRange: isize = -34;
    // Resource deadlock avoided
    pub const ResourceDeadlockAvoided: isize = -35;
    // File name too long
    pub const FileNameTooLong: isize = -36;
    // No locks available
    pub const NoLocksAvailable: isize = -37;
    // Function not implemented
    pub const FunctionNotImplemented: isize = -38;
    // Directory not empty
    pub const DirectoryNotEmpty: isize = -39;
    // Too many levels of symbolic links
    pub const TooManyLevelsOfSymbolicLinks: isize = -40;
    // Unknown error 41
    pub const UnknownError41: isize = -41;
    // No message of desired type
    pub const NoMessageOfDesiredType: isize = -42;
    // Identifier removed
    pub const IdentifierRemoved: isize = -43;
    // Channel number out of range
    pub const ChannelNumberOutOfRange: isize = -44;
    // Level 2 not synchronized
    pub const Level2NotSynchronized: isize = -45;
    // Level 3 halted
    pub const Level3Halted: isize = -46;
    // Level 3 reset
    pub const Level3Reset: isize = -47;
    // Link number out of range
    pub const LinkNumberOutOfRange: isize = -48;
    // Protocol driver not attached
    pub const ProtocolDriverNotAttached: isize = -49;
    // No CSI structure available
    pub const NoCsiStructureAvailable: isize = -50;
    // Level 2 halted
    pub const Level2Halted: isize = -51;
    // Invalid exchange
    pub const InvalidExchange: isize = -52;
    // Invalid request descriptor
    pub const InvalidRequestDescriptor: isize = -53;
    // Exchange full
    pub const ExchangeFull: isize = -54;
    // No anode
    pub const NoAnode: isize = -55;
    // Invalid request code
    pub const InvalidRequestCode: isize = -56;
    // Invalid slot
    pub const InvalidSlot: isize = -57;
    // Bad font file format
    pub const BadFontFileFormat: isize = -59;
    // Device not a stream
    pub const DeviceNotAStream: isize = -60;
    // No data available
    pub const NoDataAvailable: isize = -61;
    // Timer expired
    pub const TimerExpired: isize = -62;
    // Out of streams resources
    pub const OutOfStreamsResources: isize = -63;
    // Machine is not on the network
    pub const MachineIsNotOnTheNetwork: isize = -64;
    // Package not installed
    pub const PackageNotInstalled: isize = -65;
    // Object is remote
    pub const ObjectIsRemote: isize = -66;
    // Link has been severed
    pub const LinkHasBeenSevered: isize = -67;
    // Advertise error
    pub const AdvertiseError: isize = -68;
    // Srmount error
    pub const SrmountError: isize = -69;
    // Communication error on send
    pub const CommunicationErrorOnSend: isize = -70;
    // Protocol error
    pub const ProtocolError: isize = -71;
    // Multihop attempted
    pub const MultihopAttempted: isize = -72;
    // RFS specific error
    pub const RfsSpecificError: isize = -73;
    // Bad message
    pub const BadMessage: isize = -74;
    // Value too large for defined data type
    pub const ValueTooLargeForDefinedDataType: isize = -75;
    // Name not unique on network
    pub const NameNotUniqueOnNetwork: isize = -76;
    // File descriptor in bad state
    pub const FileDescriptorInBadState: isize = -77;
    // Remote address changed
    pub const RemoteAddressChanged: isize = -78;
    // Can not access a needed shared library
    pub const CannotAccessANeededSharedLibrary: isize = -79;
    // Accessing a corrupted shared library
    pub const AccessingACorruptedSharedLibrary: isize = -80;
    //.lib section in a.out corrupted
    pub const LibSectionInAOutCorrupted: isize = -81;
    // Attempting to link in too many shared libraries
    pub const AttemptingToLinkInTooManySharedLibraries: isize = -82;
    // Cannot exec a shared library directly
    pub const CannotExecASharedLibraryDirectly: isize = -83;
    // Invalid or incomplete multibyte or wide character
    pub const InvalidOrIncompleteMultibyteOrWideCharacter: isize = -84;
    // Interrupted system call should be restarted
    pub const InterruptedSystemCallShouldBeRestarted: isize = -85;
    // Streams pipe error
    pub const StreamsPipeError: isize = -86;
    // Too many users
    pub const TooManyUsers: isize = -87;
    // Socket operation on non-socket
    pub const SocketOperationOnNonSocket: isize = -88;
    // Destination address required
    pub const DestinationAddressRequired: isize = -89;
    // Message too long
    pub const MessageTooLong: isize = -90;
    // Protocol wrong type for socket
    pub const ProtocolWrongTypeForSocket: isize = -91;
    // Protocol not available
    pub const ProtocolNotAvailable: isize = -92;
    // Protocol not supported
    pub const ProtocolNotSupported: isize = -93;
    // Socket type not supported
    pub const SocketTypeNotSupported: isize = -94;
    // Operation not supported
    pub const OperationNotSupported: isize = -95;
    // Protocol family not supported
    pub const ProtocolFamilyNotSupported: isize = -96;
    // Address family not supported by protocol
    pub const AddressFamilyNotSupportedByProtocol: isize = -97;
    // Address already in use
    pub const AddressAlreadyInUse: isize = -98;
    // Cannot assign requested address
    pub const CannotAssignRequestedAddress: isize = -99;
    // Network is down
    pub const NetworkIsDown: isize = -100;
    // Network is unreachable
    pub const NetworkIsUnreachable: isize = -101;
    // Network dropped connection on reset
    pub const NetworkDroppedConnectionOnReset: isize = -102;
    // Software caused connection abort
    pub const SoftwareCausedConnectionAbort: isize = -103;
    // Connection reset by peer
    pub const ConnectionResetByPeer: isize = -104;
    // No buffer space available
    pub const NoBufferSpaceAvailable: isize = -105;
    // Transport endpoint is already connected
    pub const TransportEndpointIsAlreadyConnected: isize = -106;
    // Transport endpoint is not connected
    pub const TransportEndpointIsNotConnected: isize = -107;
    // Cannot send after transport endpoint shutdown
    pub const CannotSendAfterTransportEndpointShutdown: isize = -108;
    // Too many references: cannot splice
    pub const TooManyReferencesCannotSplice: isize = -109;
    // Connection timed out
    pub const ConnectionTimedOut: isize = -110;
    // Connection refused
    pub const ConnectionRefused: isize = -111;
    // Host is down
    pub const HostIsDown: isize = -112;
    // No route to host
    pub const NoRouteToHost: isize = -113;
    // Operation already in progress
    pub const OperationAlreadyInProgress: isize = -114;
    // Operation now in progress
    pub const OperationNowInProgress: isize = -115;
    // Stale file handle
    pub const StaleFileHandle: isize = -116;
    // Structure needs cleaning
    pub const StructureNeedsCleaning: isize = -117;
    // Not a XENIX named type file
    pub const NotAXenixNamedTypeFile: isize = -118;
    // No XENIX semaphores available
    pub const NoXenixSemaphoresAvailable: isize = -119;
    // Is a named type file
    pub const IsANamedTypeFile: isize = -120;
    // Remote I/O error
    pub const RemoteIOError: isize = -121;
    // Disk quota exceeded
    pub const DiskQuotaExceeded: isize = -122;
    // No medium found
    pub const NoMediumFound: isize = -123;
    // Wrong medium type
    pub const WrongMediumType: isize = -124;
    // Operation canceled
    pub const OperationCanceled: isize = -125;
    // Required key not available
    pub const RequiredKeyNotAvailable: isize = -126;
    // Key has expired
    pub const KeyHasExpired: isize = -127;
    // Key has been revoked
    pub const KeyHasBeenRevoked: isize = -128;
    // Key was rejected by service
    pub const KeyWasRejectedByService: isize = -129;
    // Owner died
    pub const OwnerDied: isize = -130;
    // State not recoverable
    pub const StateNotRecoverable: isize = -131;
    // Operation not possible due to RF-kill
    pub const OperationNotPossibleDueToRfKill: isize = -132;
    // Memory page has hardware error
    pub const MemoryPageHasHardwareError: isize = -133;
}

pub struct SyscallError;

// Result<isize, isize> is the actual type of SyscallResult, so we can return an instance directly
#[allow(non_upper_case_globals)]
impl SyscallError {
    // Success
    pub const Success: Result<isize, isize> = Err(ErrNo::Success);
    // Operation not permitted
    pub const OperationNotPermitted: Result<isize, isize> = Err(ErrNo::OperationNotPermitted);
    // No such file or directory
    pub const NoSuchFileOrDirectory: Result<isize, isize> = Err(ErrNo::NoSuchFileOrDirectory);
    // No such process
    pub const NoSuchProcess: Result<isize, isize> = Err(ErrNo::NoSuchProcess);
    // Interrupted system call
    pub const InterruptedSystemCall: Result<isize, isize> = Err(ErrNo::InterruptedSystemCall);
    // Input/output error
    pub const InputOutputError: Result<isize, isize> = Err(ErrNo::InputOutputError);
    // No such device or address
    pub const NoSuchDeviceOrAddress: Result<isize, isize> = Err(ErrNo::NoSuchDeviceOrAddress);
    // Argument list too long
    pub const ArgumentListTooLong: Result<isize, isize> = Err(ErrNo::ArgumentListTooLong);
    // Exec format error
    pub const ExecFormatError: Result<isize, isize> = Err(ErrNo::ExecFormatError);
    // Bad file descriptor
    pub const BadFileDescriptor: Result<isize, isize> = Err(ErrNo::BadFileDescriptor);
    // No child processes
    pub const NoChildProcesses: Result<isize, isize> = Err(ErrNo::NoChildProcesses);
    // Resource temporarily unavailable
    pub const ResourceTemporarilyUnavailable: Result<isize, isize> =
        Err(ErrNo::ResourceTemporarilyUnavailable);
    // Cannot allocate memory
    pub const CannotAllocateMemory: Result<isize, isize> = Err(ErrNo::CannotAllocateMemory);
    // Permission denied
    pub const PermissionDenied: Result<isize, isize> = Err(ErrNo::PermissionDenied);
    // Bad address
    pub const BadAddress: Result<isize, isize> = Err(ErrNo::BadAddress);
    // Block device required
    pub const BlockDeviceRequired: Result<isize, isize> = Err(ErrNo::BlockDeviceRequired);
    // Device or resource busy
    pub const DeviceOrResourceBusy: Result<isize, isize> = Err(ErrNo::DeviceOrResourceBusy);
    // File exists
    pub const FileExists: Result<isize, isize> = Err(ErrNo::FileExists);
    // Invalid cross-device link
    pub const InvalidCrossDeviceLink: Result<isize, isize> = Err(ErrNo::InvalidCrossDeviceLink);
    // No such device
    pub const NoSuchDevice: Result<isize, isize> = Err(ErrNo::NoSuchDevice);
    // Not a directory
    pub const NotADirectory: Result<isize, isize> = Err(ErrNo::NotADirectory);
    // Is a directory
    pub const IsADirectory: Result<isize, isize> = Err(ErrNo::IsADirectory);
    // Invalid argument
    pub const InvalidArgument: Result<isize, isize> = Err(ErrNo::InvalidArgument);
    // Too many open files in system
    pub const TooManyOpenFilesInSystem: Result<isize, isize> = Err(ErrNo::TooManyOpenFilesInSystem);
    // Too many open files
    pub const TooManyOpenFiles: Result<isize, isize> = Err(ErrNo::TooManyOpenFiles);
    // Inappropriate ioctl for device
    pub const InappropriateIoctlForDevice: Result<isize, isize> =
        Err(ErrNo::InappropriateIoctlForDevice);
    // Text file busy
    pub const TextFileBusy: Result<isize, isize> = Err(ErrNo::TextFileBusy);
    // File too large
    pub const FileTooLarge: Result<isize, isize> = Err(ErrNo::FileTooLarge);
    // No space left on device
    pub const NoSpaceLeftOnDevice: Result<isize, isize> = Err(ErrNo::NoSpaceLeftOnDevice);
    // Illegal seek
    pub const IllegalSeek: Result<isize, isize> = Err(ErrNo::IllegalSeek);
    // Read-only file system
    pub const ReadOnlyFileSystem: Result<isize, isize> = Err(ErrNo::ReadOnlyFileSystem);
    // Too many links
    pub const TooManyLinks: Result<isize, isize> = Err(ErrNo::TooManyLinks);
    // Broken pipe
    pub const BrokenPipe: Result<isize, isize> = Err(ErrNo::BrokenPipe);
    // Numerical argument out of domain
    pub const NumericalArgumentOutOfDomain: Result<isize, isize> =
        Err(ErrNo::NumericalArgumentOutOfDomain);
    // Numerical result out of range
    pub const NumericalResultOutOfRange: Result<isize, isize> =
        Err(ErrNo::NumericalResultOutOfRange);
    // Resource deadlock avoided
    pub const ResourceDeadlockAvoided: Result<isize, isize> = Err(ErrNo::ResourceDeadlockAvoided);
    // File name too long
    pub const FileNameTooLong: Result<isize, isize> = Err(ErrNo::FileNameTooLong);
    // No locks available
    pub const NoLocksAvailable: Result<isize, isize> = Err(ErrNo::NoLocksAvailable);
    // Function not implemented
    pub const FunctionNotImplemented: Result<isize, isize> = Err(ErrNo::FunctionNotImplemented);
    // Directory not empty
    pub const DirectoryNotEmpty: Result<isize, isize> = Err(ErrNo::DirectoryNotEmpty);
    // Too many levels of symbolic links
    pub const TooManyLevelsOfSymbolicLinks: Result<isize, isize> =
        Err(ErrNo::TooManyLevelsOfSymbolicLinks);
    // Unknown error 41
    pub const UnknownError41: Result<isize, isize> = Err(ErrNo::UnknownError41);
    // No message of desired type
    pub const NoMessageOfDesiredType: Result<isize, isize> = Err(ErrNo::NoMessageOfDesiredType);
    // Identifier removed
    pub const IdentifierRemoved: Result<isize, isize> = Err(ErrNo::IdentifierRemoved);
    // Channel number out of range
    pub const ChannelNumberOutOfRange: Result<isize, isize> = Err(ErrNo::ChannelNumberOutOfRange);
    // Level 2 not synchronized
    pub const Level2NotSynchronized: Result<isize, isize> = Err(ErrNo::Level2NotSynchronized);
    // Level 3 halted
    pub const Level3Halted: Result<isize, isize> = Err(ErrNo::Level3Halted);
    // Level 3 reset
    pub const Level3Reset: Result<isize, isize> = Err(ErrNo::Level3Reset);
    // Link number out of range
    pub const LinkNumberOutOfRange: Result<isize, isize> = Err(ErrNo::LinkNumberOutOfRange);
    // Protocol driver not attached
    pub const ProtocolDriverNotAttached: Result<isize, isize> =
        Err(ErrNo::ProtocolDriverNotAttached);
    // No CSI structure available
    pub const NoCsiStructureAvailable: Result<isize, isize> = Err(ErrNo::NoCsiStructureAvailable);
    // Level 2 halted
    pub const Level2Halted: Result<isize, isize> = Err(ErrNo::Level2Halted);
    // Invalid exchange
    pub const InvalidExchange: Result<isize, isize> = Err(ErrNo::InvalidExchange);
    // Invalid request descriptor
    pub const InvalidRequestDescriptor: Result<isize, isize> = Err(ErrNo::InvalidRequestDescriptor);
    // Exchange full
    pub const ExchangeFull: Result<isize, isize> = Err(ErrNo::ExchangeFull);
    // No anode
    pub const NoAnode: Result<isize, isize> = Err(ErrNo::NoAnode);
    // Invalid request code
    pub const InvalidRequestCode: Result<isize, isize> = Err(ErrNo::InvalidRequestCode);
    // Invalid slot
    pub const InvalidSlot: Result<isize, isize> = Err(ErrNo::InvalidSlot);
    // Bad font file format
    pub const BadFontFileFormat: Result<isize, isize> = Err(ErrNo::BadFontFileFormat);
    // Device not a stream
    pub const DeviceNotAStream: Result<isize, isize> = Err(ErrNo::DeviceNotAStream);
    // No data available
    pub const NoDataAvailable: Result<isize, isize> = Err(ErrNo::NoDataAvailable);
    // Timer expired
    pub const TimerExpired: Result<isize, isize> = Err(ErrNo::TimerExpired);
    // Out of streams resources
    pub const OutOfStreamsResources: Result<isize, isize> = Err(ErrNo::OutOfStreamsResources);
    // Machine is not on the network
    pub const MachineIsNotOnTheNetwork: Result<isize, isize> = Err(ErrNo::MachineIsNotOnTheNetwork);
    // Package not installed
    pub const PackageNotInstalled: Result<isize, isize> = Err(ErrNo::PackageNotInstalled);
    // Object is remote
    pub const ObjectIsRemote: Result<isize, isize> = Err(ErrNo::ObjectIsRemote);
    // Link has been severed
    pub const LinkHasBeenSevered: Result<isize, isize> = Err(ErrNo::LinkHasBeenSevered);
    // Advertise error
    pub const AdvertiseError: Result<isize, isize> = Err(ErrNo::AdvertiseError);
    // Srmount error
    pub const SrmountError: Result<isize, isize> = Err(ErrNo::SrmountError);
    // Communication error on send
    pub const CommunicationErrorOnSend: Result<isize, isize> = Err(ErrNo::CommunicationErrorOnSend);
    // Protocol error
    pub const ProtocolError: Result<isize, isize> = Err(ErrNo::ProtocolError);
    // Multihop attempted
    pub const MultihopAttempted: Result<isize, isize> = Err(ErrNo::MultihopAttempted);
    // RFS specific error
    pub const RfsSpecificError: Result<isize, isize> = Err(ErrNo::RfsSpecificError);
    // Bad message
    pub const BadMessage: Result<isize, isize> = Err(ErrNo::BadMessage);
    // Value too large for defined data type
    pub const ValueTooLargeForDefinedDataType: Result<isize, isize> =
        Err(ErrNo::ValueTooLargeForDefinedDataType);
    // Name not unique on network
    pub const NameNotUniqueOnNetwork: Result<isize, isize> = Err(ErrNo::NameNotUniqueOnNetwork);
    // File descriptor in bad state
    pub const FileDescriptorInBadState: Result<isize, isize> = Err(ErrNo::FileDescriptorInBadState);
    // Remote address changed
    pub const RemoteAddressChanged: Result<isize, isize> = Err(ErrNo::RemoteAddressChanged);
    // Can not access a needed shared library
    pub const CannotAccessANeededSharedLibrary: Result<isize, isize> =
        Err(ErrNo::CannotAccessANeededSharedLibrary);
    // Accessing a corrupted shared library
    pub const AccessingACorruptedSharedLibrary: Result<isize, isize> =
        Err(ErrNo::AccessingACorruptedSharedLibrary);
    //.lib section in a.out corrupted
    pub const LibSectionInAOutCorrupted: Result<isize, isize> =
        Err(ErrNo::LibSectionInAOutCorrupted);
    // Attempting to link in too many shared libraries
    pub const AttemptingToLinkInTooManySharedLibraries: Result<isize, isize> =
        Err(ErrNo::AttemptingToLinkInTooManySharedLibraries);
    // Cannot exec a shared library directly
    pub const CannotExecASharedLibraryDirectly: Result<isize, isize> =
        Err(ErrNo::CannotExecASharedLibraryDirectly);
    // Invalid or incomplete multibyte or wide character
    pub const InvalidOrIncompleteMultibyteOrWideCharacter: Result<isize, isize> =
        Err(ErrNo::InvalidOrIncompleteMultibyteOrWideCharacter);
    // Interrupted system call should be restarted
    pub const InterruptedSystemCallShouldBeRestarted: Result<isize, isize> =
        Err(ErrNo::InterruptedSystemCallShouldBeRestarted);
    // Streams pipe error
    pub const StreamsPipeError: Result<isize, isize> = Err(ErrNo::StreamsPipeError);
    // Too many users
    pub const TooManyUsers: Result<isize, isize> = Err(ErrNo::TooManyUsers);
    // Socket operation on non-socket
    pub const SocketOperationOnNonSocket: Result<isize, isize> =
        Err(ErrNo::SocketOperationOnNonSocket);
    // Destination address required
    pub const DestinationAddressRequired: Result<isize, isize> =
        Err(ErrNo::DestinationAddressRequired);
    // Message too long
    pub const MessageTooLong: Result<isize, isize> = Err(ErrNo::MessageTooLong);
    // Protocol wrong type for socket
    pub const ProtocolWrongTypeForSocket: Result<isize, isize> =
        Err(ErrNo::ProtocolWrongTypeForSocket);
    // Protocol not available
    pub const ProtocolNotAvailable: Result<isize, isize> = Err(ErrNo::ProtocolNotAvailable);
    // Protocol not supported
    pub const ProtocolNotSupported: Result<isize, isize> = Err(ErrNo::ProtocolNotSupported);
    // Socket type not supported
    pub const SocketTypeNotSupported: Result<isize, isize> = Err(ErrNo::SocketTypeNotSupported);
    // Operation not supported
    pub const OperationNotSupported: Result<isize, isize> = Err(ErrNo::OperationNotSupported);
    // Protocol family not supported
    pub const ProtocolFamilyNotSupported: Result<isize, isize> =
        Err(ErrNo::ProtocolFamilyNotSupported);
    // Address family not supported by protocol
    pub const AddressFamilyNotSupportedByProtocol: Result<isize, isize> =
        Err(ErrNo::AddressFamilyNotSupportedByProtocol);
    // Address already in use
    pub const AddressAlreadyInUse: Result<isize, isize> = Err(ErrNo::AddressAlreadyInUse);
    // Cannot assign requested address
    pub const CannotAssignRequestedAddress: Result<isize, isize> =
        Err(ErrNo::CannotAssignRequestedAddress);
    // Network is down
    pub const NetworkIsDown: Result<isize, isize> = Err(ErrNo::NetworkIsDown);
    // Network is unreachable
    pub const NetworkIsUnreachable: Result<isize, isize> = Err(ErrNo::NetworkIsUnreachable);
    // Network dropped connection on reset
    pub const NetworkDroppedConnectionOnReset: Result<isize, isize> =
        Err(ErrNo::NetworkDroppedConnectionOnReset);
    // Software caused connection abort
    pub const SoftwareCausedConnectionAbort: Result<isize, isize> =
        Err(ErrNo::SoftwareCausedConnectionAbort);
    // Connection reset by peer
    pub const ConnectionResetByPeer: Result<isize, isize> = Err(ErrNo::ConnectionResetByPeer);
    // No buffer space available
    pub const NoBufferSpaceAvailable: Result<isize, isize> = Err(ErrNo::NoBufferSpaceAvailable);
    // Transport endpoint is already connected
    pub const TransportEndpointIsAlreadyConnected: Result<isize, isize> =
        Err(ErrNo::TransportEndpointIsAlreadyConnected);
    // Transport endpoint is not connected
    pub const TransportEndpointIsNotConnected: Result<isize, isize> =
        Err(ErrNo::TransportEndpointIsNotConnected);
    // Cannot send after transport endpoint shutdown
    pub const CannotSendAfterTransportEndpointShutdown: Result<isize, isize> =
        Err(ErrNo::CannotSendAfterTransportEndpointShutdown);
    // Too many references: cannot splice
    pub const TooManyReferencesCannotSplice: Result<isize, isize> =
        Err(ErrNo::TooManyReferencesCannotSplice);
    // Connection timed out
    pub const ConnectionTimedOut: Result<isize, isize> = Err(ErrNo::ConnectionTimedOut);
    // Connection refused
    pub const ConnectionRefused: Result<isize, isize> = Err(ErrNo::ConnectionRefused);
    // Host is down
    pub const HostIsDown: Result<isize, isize> = Err(ErrNo::HostIsDown);
    // No route to host
    pub const NoRouteToHost: Result<isize, isize> = Err(ErrNo::NoRouteToHost);
    // Operation already in progress
    pub const OperationAlreadyInProgress: Result<isize, isize> =
        Err(ErrNo::OperationAlreadyInProgress);
    // Operation now in progress
    pub const OperationNowInProgress: Result<isize, isize> = Err(ErrNo::OperationNowInProgress);
    // Stale file handle
    pub const StaleFileHandle: Result<isize, isize> = Err(ErrNo::StaleFileHandle);
    // Structure needs cleaning
    pub const StructureNeedsCleaning: Result<isize, isize> = Err(ErrNo::StructureNeedsCleaning);
    // Not a XENIX named type file
    pub const NotAXenixNamedTypeFile: Result<isize, isize> = Err(ErrNo::NotAXenixNamedTypeFile);
    // No XENIX semaphores available
    pub const NoXenixSemaphoresAvailable: Result<isize, isize> =
        Err(ErrNo::NoXenixSemaphoresAvailable);
    // Is a named type file
    pub const IsANamedTypeFile: Result<isize, isize> = Err(ErrNo::IsANamedTypeFile);
    // Remote I/O error
    pub const RemoteIOError: Result<isize, isize> = Err(ErrNo::RemoteIOError);
    // Disk quota exceeded
    pub const DiskQuotaExceeded: Result<isize, isize> = Err(ErrNo::DiskQuotaExceeded);
    // No medium found
    pub const NoMediumFound: Result<isize, isize> = Err(ErrNo::NoMediumFound);
    // Wrong medium type
    pub const WrongMediumType: Result<isize, isize> = Err(ErrNo::WrongMediumType);
    // Operation canceled
    pub const OperationCanceled: Result<isize, isize> = Err(ErrNo::OperationCanceled);
    // Required key not available
    pub const RequiredKeyNotAvailable: Result<isize, isize> = Err(ErrNo::RequiredKeyNotAvailable);
    // Key has expired
    pub const KeyHasExpired: Result<isize, isize> = Err(ErrNo::KeyHasExpired);
    // Key has been revoked
    pub const KeyHasBeenRevoked: Result<isize, isize> = Err(ErrNo::KeyHasBeenRevoked);
    // Key was rejected by service
    pub const KeyWasRejectedByService: Result<isize, isize> = Err(ErrNo::KeyWasRejectedByService);
    // Owner died
    pub const OwnerDied: Result<isize, isize> = Err(ErrNo::OwnerDied);
    // State not recoverable
    pub const StateNotRecoverable: Result<isize, isize> = Err(ErrNo::StateNotRecoverable);
    // Operation not possible due to RF-kill
    pub const OperationNotPossibleDueToRfKill: Result<isize, isize> =
        Err(ErrNo::OperationNotPossibleDueToRfKill);
    // Memory page has hardware error
    pub const MemoryPageHasHardwareError: Result<isize, isize> =
        Err(ErrNo::MemoryPageHasHardwareError);
}
