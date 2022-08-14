use std::{
    thread,
    time::{Duration, Instant},
};

const WAIT_TIME: Duration = Duration::from_secs(2);

pub static mut LOG: Option<Log> = None;

pub struct Log {
    pub message: String,
    pub timer: Option<Instant>,
}

pub fn init() {
    if unsafe { LOG.is_some() } {
        return;
    }

    unsafe {
        LOG = Some(Log {
            message: String::new(),
            timer: None,
        });
    }

    thread::spawn(|| loop {
        thread::sleep(Duration::from_millis(1));
        if let Some(log) = unsafe { &mut LOG } {
            if let Some(timer) = log.timer {
                if timer.elapsed() >= WAIT_TIME {
                    log.timer = None;
                    log.message = String::new();
                }
            }
        }
    });
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use std::time::Instant;
        use $crate::log::LOG;

        unsafe {
            let log = LOG.as_mut().unwrap();
            log.message = format_args!($($arg)*).to_string();
            log.timer = Some(Instant::now());
        }
    }
    };
}

pub fn message() -> Option<&'static str> {
    if let Some(log) = unsafe { &LOG } {
        if !log.message.is_empty() {
            return Some(log.message.as_str());
        }
    }
    None
}
