//! TODO: Cleanup
//!
//!
use std::{
    collections::{hash_map::Entry, HashMap},
    mem,
    sync::Mutex,
    time::{Duration, Instant},
};

static mut EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

///Print the profiler events.
pub fn print() {
    let events = unsafe { mem::take(EVENTS.get_mut().unwrap()) };

    if events.is_empty() {
        return;
    }

    let mut map: HashMap<Location, Vec<Event>> = HashMap::new();

    for event in events {
        match map.entry(event.location.clone()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(event.clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![event.clone()]);
            }
        }
    }

    let mut scores = Vec::new();

    for (k, v) in map.iter() {
        let mut mean = Duration::default();
        let mut min = unsafe { v.get(0).unwrap_or(&Event::default()).elapsed() };
        let mut max = Duration::default();

        for event in v {
            let elapsed = unsafe { event.elapsed() };

            if elapsed < min {
                min = elapsed;
            }

            if elapsed > max {
                max = elapsed;
            }

            mean += elapsed;
        }

        scores.push(Score {
            name: k.name,
            file: k.file,
            line: k.line,
            total: mean,
            mean: Duration::from_secs_f64(mean.as_secs_f64() / v.len() as f64),
            min,
            max,
            count: v.len(),
        });
    }

    for score in scores {
        println!(
            "{} ({} runs) {}:{}",
            score.name, score.count, score.file, score.line,
        );
        println!("   - total: {:?}", score.total);
        println!("   - mean: {:?}", score.mean);
        println!("   - min: {:?}", score.min);
        println!("   - max: {:?}\n", score.max);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Event {
    pub location: Location,
    pub start: Option<Instant>,
    pub end: Option<Instant>,
}

impl Event {
    pub unsafe fn elapsed(&self) -> Duration {
        self.end
            .unwrap_unchecked()
            .duration_since(self.start.unwrap_unchecked())
    }
}

#[derive(Debug, Default)]
pub struct Score {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub total: Duration,
    pub mean: Duration,
    pub min: Duration,
    pub max: Duration,
    pub count: usize,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug, Default)]
pub struct Location {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
}

impl Drop for Event {
    #[inline(always)]
    fn drop(&mut self) {
        self.end = Some(Instant::now());

        let event = std::mem::take(self);

        unsafe { EVENTS.lock().unwrap().push(event) };
    }
}

#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

#[macro_export]
macro_rules! profile {
    () => {
        use $crate::profiler::*;
        let _event;

        if cfg!(feature = "profile") {
            _event = Event {
                location: Location {
                    name: $crate::function!(),
                    file: file!(),
                    line: line!(),
                },
                start: Some(std::time::Instant::now()),
                end: None,
            };
        }
    };
    ($name:expr) => {
        use $crate::profiler::*;
        let _event;

        if cfg!(feature = "profile") {
            _event = Event {
                location: Location {
                    name: $name,
                    file: file!(),
                    line: line!(),
                },
                start: Some(std::time::Instant::now()),
                end: None,
            };
        }
    };
}
