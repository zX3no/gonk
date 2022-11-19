use std::{
    collections::{hash_map::Entry, HashMap},
    sync::RwLock,
    time::{Duration, Instant},
};

pub static mut EVENTS: Option<RwLock<HashMap<Location, Vec<Event>>>> = None;

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
struct Score {
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
        let mut map = unsafe { EVENTS.as_mut().unwrap().write().unwrap() };
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

#[inline(always)]
pub fn profile(_name: &'static str) {
    #[cfg(feature = "profile")]
    let _drop = Dropper {
        event: Event {
            start: Some(std::time::Instant::now()),
            end: None,
        },
        location: Location {
            name: _name,
            file: file!(),
            line: line!(),
        },
    };
}

#[macro_export]
macro_rules! profile {
    () => {
        $crate::profiler::profile($crate::function!());
        // $crate::profile!($crate::function!());
    };
    ($name:expr) => {
        $crate::profiler::profile($name);
        // let _drop = $crate::profiler::Dropper {
        //     event: $crate::profiler::Event {
        //         start: Some(std::time::Instant::now()),
        //         end: None,
        //     },
        //     location: $crate::profiler::Location {
        //         name: $name,
        //         file: file!(),
        //         line: line!(),
        //     },
        // };
    };
}

pub fn init() {
    unsafe {
        EVENTS = Some(RwLock::new(HashMap::default()));
    }
}

pub fn log() {
    let events = unsafe { EVENTS.as_ref().unwrap().read().unwrap() };

    if events.is_empty() {
        return;
    }

    let mut scores = Vec::new();

    for (k, v) in events.iter() {
        let mut mean = Duration::default();
        let mut min = Duration::default();
        let mut max = Duration::default();
        for event in v {
            let elapsed = unsafe { event.elapsed() };

            if elapsed < min || min.is_zero() {
                min = elapsed;
            }

            if elapsed > max || max.is_zero() {
                max = elapsed;
            }

            mean += elapsed;
        }

        scores.push(Score {
            name: k.name,
            file: k.file,
            line: k.line,
            mean: mean.checked_div(v.len() as u32).unwrap_or_default(),
            min,
            max,
            count: v.len(),
        });
    }

    let mut string = String::new();
    for score in scores {
        string.push_str(&format!(
            "{} ({} runs) {}:{}\n",
            score.name, score.count, score.file, score.line,
        ));
        string.push_str(&format!("   - mean: {:?}\n", score.mean));
        string.push_str(&format!("   - min: {:?}\n", score.min));
        string.push_str(&format!("   - max: {:?}\n\n", score.max));
    }

    println!("{}", string);
}
