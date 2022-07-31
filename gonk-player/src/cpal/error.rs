use std::{error::Error, fmt};

/// The requested host, although supported on this platform, is unavailable.
#[derive(Clone, Debug)]
pub struct HostUnavailable;

impl fmt::Display for HostUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the requested host is unavailable")
    }
}

impl Error for HostUnavailable {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// Some error has occurred that is specific to the backend from which it was produced.
///
/// This error is often used as a catch-all in cases where:
///
/// - It is unclear exactly what error might be produced by the backend API.
/// - It does not make sense to add a variant to the enclosing error type.
/// - No error was expected to occur at all, but we return an error to avoid the possibility of a
///   `panic!` caused by some unforeseen or unknown reason.
///
/// **Note:** If you notice a `BackendSpecificError` that you believe could be better handled in a
/// cross-platform manner, please create an issue or submit a pull request with a patch that adds
/// the necessary error variant to the appropriate error enum.
#[derive(Clone, Debug)]
pub struct BackendSpecificError {
    pub description: String,
}

impl fmt::Display for BackendSpecificError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A backend-specific error has occurred: {}",
            self.description
        )
    }
}

impl Error for BackendSpecificError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// An error that might occur while attempting to enumerate the available devices on a system.
#[derive(Debug)]
pub enum DevicesError {
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for DevicesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DevicesError::BackendSpecific { err } => write!(f, "{}", err),
        }
    }
}

impl Error for DevicesError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// An error that may occur while attempting to retrieve a device name.
#[derive(Debug)]
pub enum DeviceNameError {
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for DeviceNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceNameError::BackendSpecific { err } => write!(f, "{}", err),
        }
    }
}

impl Error for DeviceNameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// Error that can happen when enumerating the list of supported formats.
#[derive(Debug)]
pub enum SupportedStreamConfigsError {
    /// The device no longer exists. This can happen if the device is disconnected while the
    /// program is running.
    DeviceNotAvailable,
    /// We called something the C-Layer did not understand
    InvalidArgument,
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for SupportedStreamConfigsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match self {
            SupportedStreamConfigsError::DeviceNotAvailable => {
                "The requested device is no longer available. For example, it has been unplugged."
            }
            SupportedStreamConfigsError::InvalidArgument => "Invalid argument passed to the backend. For example, this happens when trying to read capture capabilities when the device does not support it.",
            SupportedStreamConfigsError::BackendSpecific { err } => err.description.as_str(),
        };
        write!(f, "{err}")
    }
}

impl Error for SupportedStreamConfigsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// May occur when attempting to request the default input or output stream format from a `Device`.
#[derive(Debug)]
pub enum DefaultStreamConfigError {
    /// The device no longer exists. This can happen if the device is disconnected while the
    /// program is running.
    DeviceNotAvailable,
    /// Returned if e.g. the default input format was requested on an output-only audio device.
    StreamTypeNotSupported,
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for DefaultStreamConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match self {
            DefaultStreamConfigError::DeviceNotAvailable => {
                "The requested device is no longer available. For example, it has been unplugged."
            }
            DefaultStreamConfigError::StreamTypeNotSupported => {
                "The requested stream type is not supported by the device."
            }
            DefaultStreamConfigError::BackendSpecific { err } => err.description.as_str(),
        };
        write!(f, "{err}")
    }
}

impl Error for DefaultStreamConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// Error that can happen when creating a `Stream`.
#[derive(Debug)]
pub enum BuildStreamError {
    /// The device no longer exists. This can happen if the device is disconnected while the
    /// program is running.
    DeviceNotAvailable,
    /// The specified stream configuration is not supported.
    StreamConfigNotSupported,
    /// We called something the C-Layer did not understand
    ///
    /// On ALSA device functions called with a feature they do not support will yield this. E.g.
    /// Trying to use capture capabilities on an output only format yields this.
    InvalidArgument,
    /// Occurs if adding a new Stream ID would cause an integer overflow.
    StreamIdOverflow,
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for BuildStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match self {
            BuildStreamError::DeviceNotAvailable => {
                "The requested device is no longer available. For example, it has been unplugged."
            }
            BuildStreamError::StreamConfigNotSupported => {
                "The requested stream configuration is not supported by the device."
            }
            BuildStreamError::InvalidArgument => {
                "The requested device does not support this capability (invalid argument)"
            }
            BuildStreamError::StreamIdOverflow => "Adding a new stream ID would cause an overflow",
            BuildStreamError::BackendSpecific { err } => err.description.as_str(),
        };
        write!(f, "{err}")
    }
}

impl Error for BuildStreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// Errors that might occur when calling `play_stream`.
///
/// As of writing this, only macOS may immediately return an error while calling this method. This
/// is because both the alsa and wasapi backends only enqueue these commands and do not process
/// them immediately.
#[derive(Debug)]
pub enum PlayStreamError {
    /// The device associated with the stream is no longer available.
    DeviceNotAvailable,
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for PlayStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match self {
            PlayStreamError::DeviceNotAvailable => {
                "the device associated with the stream is no longer available"
            }
            PlayStreamError::BackendSpecific { err } => err.description.as_str(),
        };
        write!(f, "{err}")
    }
}

impl Error for PlayStreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// Errors that might occur when calling `pause_stream`.
///
/// As of writing this, only macOS may immediately return an error while calling this method. This
/// is because both the alsa and wasapi backends only enqueue these commands and do not process
/// them immediately.
#[derive(Debug)]
pub enum PauseStreamError {
    /// The device associated with the stream is no longer available.
    DeviceNotAvailable,
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for PauseStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match self {
            PauseStreamError::DeviceNotAvailable => {
                "the device associated with the stream is no longer available"
            }
            PauseStreamError::BackendSpecific { err } => err.description.as_str(),
        };
        write!(f, "{err}")
    }
}

impl Error for PauseStreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

/// Errors that might occur while a stream is running.
#[derive(Debug)]
pub enum StreamError {
    /// The device no longer exists. This can happen if the device is disconnected while the
    /// program is running.
    DeviceNotAvailable,
    /// See the `BackendSpecificError` docs for more information about this error variant.
    BackendSpecific { err: BackendSpecificError },
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match self {
            StreamError::DeviceNotAvailable => {
                "The requested device is no longer available. For example, it has been unplugged."
            }
            StreamError::BackendSpecific { err } => err.description.as_str(),
        };
        write!(f, "{err}")
    }
}

impl Error for StreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

impl From<BackendSpecificError> for StreamError {
    fn from(err: BackendSpecificError) -> Self {
        Self::BackendSpecific { err }
    }
}
