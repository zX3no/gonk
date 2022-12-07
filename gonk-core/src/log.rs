//! TODO: Cleanup
//!
//!
use crate::Lazy;
use std::{sync::Once, time::Instant};

#[doc(hidden)]
pub static ONCE: Once = Once::new();

#[doc(hidden)]
pub static mut LOG: Lazy<Log> = Lazy::new(|| Log {
    messages: Vec::new(),
    timer: Instant::now(),
});

#[doc(hidden)]
pub struct Log {
    pub messages: Vec<String>,
    pub timer: Instant,
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use $crate::log::{LOG, ONCE};
        use std::time::{Instant, Duration};
        use std::thread;

        ONCE.call_once(|| {
            thread::spawn(|| loop {
                thread::sleep(Duration::from_millis(16));
                unsafe {
                    if LOG.timer.elapsed() >= Duration::from_millis(2500) {
                        LOG.messages.pop();
                        LOG.timer = Instant::now();
                    }
                }
            });
        });

        unsafe {
            LOG.messages.push(format_args!($($arg)*).to_string());
        }
    }
    };
}

pub fn clear() {
    unsafe {
        LOG.messages = Vec::new();
        LOG.timer = Instant::now();
    }
}

pub fn last_message() -> Option<&'static str> {
    if let Some(message) = unsafe { LOG.messages.last() } {
        Some(message.as_str())
    } else {
        None
    }
}
