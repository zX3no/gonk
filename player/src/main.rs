#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc,
    non_upper_case_globals,
    non_snake_case
)]

mod windows;
pub use windows::*;

fn main() {
    let mut handle = unsafe { create_stream() };

    let mut phase: f32 = 0.0;
    let pitch: f32 = 440.0;
    let gain: f32 = 0.1;
    let step = std::f32::consts::PI * 2.0 * pitch / handle.sample_rate as f32;

    loop {
        let smp = phase.sin() * gain;
        phase += step;
        if phase >= std::f32::consts::PI * 2.0 {
            phase -= std::f32::consts::PI * 2.0
        }

        while handle.prod.push(smp * 0.1).is_err() {
            //Don't push when the buffer is full
        }
    }
}
