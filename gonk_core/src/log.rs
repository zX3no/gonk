//! TODO: Cleanup
//!
//!
use std::{
    sync::Once,
    time::{Duration, Instant},
};

#[doc(hidden)]
pub static ONCE: Once = Once::new();

#[doc(hidden)]
pub static mut LOG: Log = Log::new();

#[doc(hidden)]
pub const MESSAGE_COOLDOWN: Duration = Duration::from_millis(1500);

#[doc(hidden)]
#[derive(Debug)]
pub struct Log {
    pub messages: Vec<(String, Instant)>,
}

impl Log {
    pub const fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use $crate::log::{LOG, ONCE, MESSAGE_COOLDOWN};
        use std::time::{Instant, Duration};
        use std::thread;

        ONCE.call_once(|| {
            thread::spawn(|| loop {
                thread::sleep(Duration::from_millis(16));

                if let Some((_, instant)) = unsafe { LOG.messages.last() } {
                    if instant.elapsed() >=  MESSAGE_COOLDOWN {
                        unsafe { LOG.messages.pop() };

                        //Reset the next messages since they run paralell.
                        //Not a good way of doing this.
                        if let Some((_, instant)) = unsafe { LOG.messages.last_mut() } {
                            *instant = Instant::now();
                        }
                    }
                }
            });
        });

        unsafe {
            LOG.messages.push((format_args!($($arg)*).to_string(), Instant::now()));
        }
    }
    };
}

pub fn clear() {
    unsafe {
        LOG.messages = Vec::new();
    }
}

pub fn last_message() -> Option<&'static str> {
    if let Some((message, _)) = unsafe { LOG.messages.last() } {
        Some(message.as_str())
    } else {
        None
    }
}
