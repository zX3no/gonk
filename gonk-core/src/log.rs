use std::{fs, sync::Once, time::Instant};

use crate::gonk_path;

pub static ONCE: Once = Once::new();

pub static mut LOG: Log = Log {
    message: String::new(),
    timer: None,
};

//TODO: Change this to Vec<String>.
//Pop a message every 2500ms.
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

pub static mut ERRORS: Vec<String> = Vec::new();

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        use $crate::log::{ERRORS};

        unsafe {
            ERRORS.push(format_args!($($arg)*).to_string());
        }
    }
    };
}

pub fn take_errors() -> usize{
    unsafe {
        let len = ERRORS.len();
        ERRORS = Vec::new();
        len
    }
}

pub fn write_errors() {
    unsafe {
        dbg!(&ERRORS);
        if !ERRORS.is_empty() {
            let path = gonk_path().join("gonk.log");
            let errors = ERRORS.join("\n");
            fs::write(path, errors).unwrap();
        }
    }
}
