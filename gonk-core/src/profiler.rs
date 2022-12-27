//! TODO: Cleanup
//!
//!
use crate::Lazy;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::RwLock,
    time::{Duration, Instant},
};

static mut EVENTS: Lazy<RwLock<HashMap<Location, Vec<Event>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug, Default)]
pub struct Event {
    pub start: Option<Instant>,
    pub end: Option<Instant>,
}

impl Event {
    /// # Safety
    pub unsafe fn elapsed(&self) -> Duration {
        self.end
            .unwrap_unchecked()
            .duration_since(self.start.unwrap_unchecked())
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug, Default)]
pub struct Location {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
}

#[derive(Debug)]
pub struct Score {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub mean: Duration,
    pub min: Duration,
    pub max: Duration,
    pub count: usize,
}

pub struct Dropper {
    pub event: Event,
    pub location: Location,
}
impl Drop for Dropper {
    #[inline(always)]
    fn drop(&mut self) {
        let mut map = unsafe { EVENTS.write().unwrap() };
        self.event.end = Some(Instant::now());
        match map.entry(self.location.clone()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(self.event.clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![self.event.clone()]);
            }
        }
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
        let _drop;

        if cfg!(feature = "profile") {
            _drop = Dropper {
                event: Event {
                    start: Some(std::time::Instant::now()),
                    end: None,
                },
                location: Location {
                    name: $crate::function!(),
                    file: file!(),
                    line: line!(),
                },
            };
        }
    };
    ($name:expr) => {
        use $crate::profiler::*;
        if !cfg!(feature = "profiler") {
            return todo!();
        }
        let _drop = Dropper {
            event: Event {
                start: Some(std::time::Instant::now()),
                end: None,
            },
            location: Location {
                name: $name,
                file: file!(),
                line: line!(),
            },
        };
    };
}

///Print the profiler events.
pub fn print() {
    let events = unsafe { EVENTS.read().unwrap() };

    if events.is_empty() {
        return;
    }

    let mut scores = Vec::new();

    for (k, v) in events.iter() {
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
        println!("   - mean: {:?}", score.mean);
        println!("   - min: {:?}", score.min);
        println!("   - max: {:?}\n", score.max);
    }
}
