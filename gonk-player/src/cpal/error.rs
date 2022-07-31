use std::{error::Error, fmt};

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

#[derive(Debug)]
pub enum DevicesError {
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

#[derive(Debug)]
pub enum DeviceNameError {
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

#[derive(Debug)]
pub enum SupportedStreamConfigsError {
    DeviceNotAvailable,

    InvalidArgument,

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

#[derive(Debug)]
pub enum DefaultStreamConfigError {
    DeviceNotAvailable,

    StreamTypeNotSupported,

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

#[derive(Debug)]
pub enum BuildStreamError {
    DeviceNotAvailable,

    StreamConfigNotSupported,

    InvalidArgument,

    StreamIdOverflow,

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

#[derive(Debug)]
pub enum PlayStreamError {
    DeviceNotAvailable,

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

#[derive(Debug)]
pub enum PauseStreamError {
    DeviceNotAvailable,

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

#[derive(Debug)]
pub enum StreamError {
    DeviceNotAvailable,

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
