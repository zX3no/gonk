use std::{mem, vec::IntoIter};

#[inline]
const fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[inline]
const fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + t * (b - a);
}

pub struct SampleRateConverter {
    /// The iterator that gives us samples.
    buffer: IntoIter<f32>,

    ///Input sample rate - interpolation factor.
    input: u32,
    ///Output sample rate - decimation factor.
    output: u32,

    /// One sample per channel, extracted from `input`.
    current_frame: Vec<f32>,
    /// Position of `current_sample` modulo `from`.
    ///
    /// `0..input / gcd`
    current_frame_pos_in_chunk: u32,

    /// The samples right after `current_sample` (one per channel), extracted from `input`.
    next_frame: Vec<f32>,
    /// The position of the next sample that the iterator should return, modulo `to`.
    /// This counter is incremented (modulo `to`) every time the iterator is called.
    ///
    /// `0..output / gcd`
    next_output_frame_pos_in_chunk: u32,

    output_buffer: Option<f32>,
}

impl SampleRateConverter {
    pub fn new(mut buffer: IntoIter<f32>, input: u32, output: u32) -> SampleRateConverter {
        debug_assert!(input >= 1);
        debug_assert!(output >= 1);

        let gcd = gcd(input, output);
        let (current_frame, next_frame) = if input == output {
            (Vec::new(), Vec::new())
        } else {
            (
                vec![buffer.next().unwrap(), buffer.next().unwrap()],
                vec![buffer.next().unwrap(), buffer.next().unwrap()],
            )
        };

        SampleRateConverter {
            buffer,
            input: input / gcd,
            output: output / gcd,
            current_frame_pos_in_chunk: 0,
            next_output_frame_pos_in_chunk: 0,
            current_frame,
            next_frame,
            output_buffer: None,
        }
    }

    fn next_input_frame(&mut self) {
        self.current_frame = mem::take(&mut self.next_frame);

        if let Some(sample) = self.buffer.next() {
            self.next_frame.push(sample);
        }

        if let Some(sample) = self.buffer.next() {
            self.next_frame.push(sample);
        }

        self.current_frame_pos_in_chunk += 1;
    }

    pub fn next(&mut self) -> Option<f32> {
        if self.input == self.output {
            return self.buffer.next();
        } else if let Some(sample) = self.output_buffer.take() {
            return Some(sample);
        }

        // The frame we are going to return from this function will be a linear interpolation
        // between `self.current_frame` and `self.next_frame`.
        if self.next_output_frame_pos_in_chunk == self.output {
            // If we jump to the next frame, we reset the whole state.
            self.next_output_frame_pos_in_chunk = 0;

            self.next_input_frame();
            while self.current_frame_pos_in_chunk != self.input {
                self.next_input_frame();
            }
            self.current_frame_pos_in_chunk = 0;
        } else {
            // Finding the position of the first sample of the linear interpolation.
            let req_left_sample =
                (self.input * self.next_output_frame_pos_in_chunk / self.output) % self.input;

            // Advancing `self.current_frame`, `self.next_frame` and
            // `self.current_frame_pos_in_chunk` until the latter variable
            // matches `req_left_sample`.
            while self.current_frame_pos_in_chunk != req_left_sample {
                self.next_input_frame();
                debug_assert!(self.current_frame_pos_in_chunk < self.input);
            }
        }

        // Merging `self.current_frame` and `self.next_frame` into `self.output_buffer`.
        // Note that `self.output_buffer` can be truncated if there is not enough data in
        // `self.next_frame`.
        let numerator = (self.input * self.next_output_frame_pos_in_chunk) % self.output;

        // Incrementing the counter for the next iteration.
        self.next_output_frame_pos_in_chunk += 1;

        if self.current_frame.is_empty() && self.next_frame.is_empty() {
            return None;
        }

        if self.next_frame.is_empty() {
            let r = self.current_frame.remove(0);
            self.output_buffer = self.current_frame.get(0).cloned();
            self.current_frame.clear();
            Some(r)
        } else {
            let ratio = numerator as f32 / self.output as f32;
            self.output_buffer = Some(lerp(self.current_frame[1], self.next_frame[1], ratio));
            Some(lerp(self.current_frame[0], self.next_frame[0], ratio))
        }
    }
}
