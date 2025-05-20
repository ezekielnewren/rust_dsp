use std::f32::consts::PI;
use num_complex::Complex32;

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
