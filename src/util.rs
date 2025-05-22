use std::f32::consts::PI;
use num_complex::Complex32;
use crate::block::FIRFilter;

#[derive(Default)]
pub struct BufferBank<T> {
    buff0: Vec<T>,
    buff1: Vec<T>,
    direction: bool,
}


impl<T> BufferBank<T> {
    pub fn swap(&mut self) -> (&mut Vec<T>, &mut Vec<T>) {
        let result = if self.direction {
            (&mut self.buff0, &mut self.buff1)
        } else {
            (&mut self.buff1, &mut self.buff0)
        };
        self.direction = !self.direction;
        result
    }
}


pub fn sinc<T: SincArg>(x: T) -> T {
    x.sinc()
}

pub trait SincArg: Copy {
    fn sinc(self) -> Self;
}

impl SincArg for f32 {
    fn sinc(self) -> Self {
        if self == 0f32 {
            1f32
        } else {
            let t = self * PI;
            t.sin() / t
        }
    }
}

impl SincArg for Complex32 {
    fn sinc(self) -> Self {
        if self.re == 0f32 && self.im == 0f32 {
            Complex32::new(1f32, 0f32)
        } else {
            let t = self * PI;
            t.sin() / t
        }
    }
}


pub unsafe fn resize_unchecked<T>(vec: &mut Vec<T>, new_length: usize) {
    if vec.capacity() < new_length {
        vec.reserve(new_length - vec.capacity());
    }
    if vec.len() != new_length {
        vec.set_len(new_length);
    }
}


pub fn lowpass_taps(cutoff: f32, num_taps: usize) -> Vec<f32> {
    let m = num_taps as isize - 1;

    let mut taps = Vec::new();

    for n in 0..num_taps as isize {
        let centered = n - m / 2;
        let sinc_val = (2.0 * cutoff * centered as f32).sinc();

        let window = 0.54 - 0.46 * ((2.0 * PI * n as f32) / m as f32).cos();
        taps.push(sinc_val * window);
    }
    
    taps
}


pub fn lowpass_real(sample_rate: u32, cutoff_hz: f32, num_taps: usize) -> FIRFilter<f32> {
    let normalized_frequency_cutoff = cutoff_hz / sample_rate as f32;
    let taps = lowpass_taps(normalized_frequency_cutoff, num_taps);
    FIRFilter::new(taps)
}

pub fn lowpass_complex(sample_rate: u32, cutoff_hz: f32, num_taps: usize) -> FIRFilter<Complex32> {
    let normalized_frequency_cutoff = cutoff_hz / sample_rate as f32;
    let taps = lowpass_taps(normalized_frequency_cutoff, num_taps);
    let complex_taps = taps.iter().copied().map(|r| Complex32::new(r, 0.0)).collect();
    FIRFilter::new(complex_taps)
}
