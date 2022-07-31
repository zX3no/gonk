use std::mem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    I16,

    U16,

    F32,
}

impl SampleFormat {
    #[inline]
    pub fn sample_size(&self) -> usize {
        match *self {
            SampleFormat::I16 => mem::size_of::<i16>(),
            SampleFormat::U16 => mem::size_of::<u16>(),
            SampleFormat::F32 => mem::size_of::<f32>(),
        }
    }
}

#[allow(clippy::missing_safety_doc)]

pub unsafe trait Sample: Copy + Clone {
    const FORMAT: SampleFormat;

    fn to_f32(&self) -> f32;

    fn to_i16(&self) -> i16;

    fn to_u16(&self) -> u16;

    fn from<S>(s: &S) -> Self
    where
        S: Sample;
}

unsafe impl Sample for u16 {
    const FORMAT: SampleFormat = SampleFormat::U16;

    #[inline]
    fn to_f32(&self) -> f32 {
        self.to_i16().to_f32()
    }

    #[inline]
    fn to_i16(&self) -> i16 {
        (*self as i16).wrapping_add(i16::MIN)
    }

    #[inline]
    fn to_u16(&self) -> u16 {
        *self
    }

    #[inline]
    fn from<S>(sample: &S) -> Self
    where
        S: Sample,
    {
        sample.to_u16()
    }
}

unsafe impl Sample for i16 {
    const FORMAT: SampleFormat = SampleFormat::I16;

    #[inline]
    fn to_f32(&self) -> f32 {
        if *self < 0 {
            *self as f32 / -(i16::MIN as f32)
        } else {
            *self as f32 / i16::MAX as f32
        }
    }

    #[inline]
    fn to_i16(&self) -> i16 {
        *self
    }

    #[inline]
    fn to_u16(&self) -> u16 {
        self.wrapping_add(i16::MIN) as u16
    }

    #[inline]
    fn from<S>(sample: &S) -> Self
    where
        S: Sample,
    {
        sample.to_i16()
    }
}
const F32_TO_16BIT_INT_MULTIPLIER: f32 = u16::MAX as f32 * 0.5;
unsafe impl Sample for f32 {
    const FORMAT: SampleFormat = SampleFormat::F32;

    #[inline]
    fn to_f32(&self) -> f32 {
        *self
    }

    #[inline]
    fn to_i16(&self) -> i16 {
        if *self >= 0.0 {
            (*self * i16::MAX as f32) as i16
        } else {
            (-*self * i16::MIN as f32) as i16
        }
    }

    #[inline]
    fn to_u16(&self) -> u16 {
        self.mul_add(F32_TO_16BIT_INT_MULTIPLIER, F32_TO_16BIT_INT_MULTIPLIER)
            .round() as u16
    }

    #[inline]
    fn from<S>(sample: &S) -> Self
    where
        S: Sample,
    {
        sample.to_f32()
    }
}
