use std::{sync::Once, time::Instant};

pub static ONCE: Once = Once::new();

pub static mut LOG: Log = Log {
    message: String::new(),
    timer: None,
};

pub struct Log {
    pub message: String,
    pub timer: Option<Instant>,
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use $crate::log::{LOG, ONCE};
        use std::time::{Instant, Duration};
        use std::thread;

        ONCE.call_once(|| {
            thread::spawn(|| loop {
                thread::sleep(Duration::from_millis(1));
                unsafe {
                    if let Some(timer) = LOG.timer {
                        if timer.elapsed() >= Duration::from_millis(2500) {
                            LOG.timer = None;
                            LOG.message = String::new();
                        }
                    }
                }
            });
        });

        unsafe {
            LOG.message = format_args!($($arg)*).to_string();
            LOG.timer = Some(Instant::now());
        }
    }
    };
}

pub fn message() -> Option<&'static str> {
    unsafe {
        if !LOG.message.is_empty() {
            return Some(LOG.message.as_str());
        }
        None
    }
}
