use std::f32::consts::PI;
use num_complex::Complex32;


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
