use std::error::Error;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use num_complex::{Complex32, Complex64};
use num_traits::{One, Zero};
use num_traits::real::Real;

pub trait Source<I> {
    fn read(&mut self, dst: &mut Vec<I>) -> Result<(), Box<dyn Error>>;
}

pub trait Filter<I, O> {
    fn filter(&mut self, input: &[I], output: &mut Vec<O>) -> Result<(), Box<dyn Error>>;
}

pub trait Sink<O> {
    fn write(&mut self, src: &[O]) -> Result<(), Box<dyn Error>>;
}

pub trait Arithmetic:
Add<Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Div<Output = Self>
+ AddAssign
+ SubAssign
+ MulAssign
+ DivAssign
+ PartialEq
+ Zero
+ One
+ Clone
+ Copy
{}

impl<T> Arithmetic for T where
    T: Add<Output = T>
    + Sub<Output = T>
    + Mul<Output = T>
    + Div<Output = T>
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + PartialEq
    + Zero
    + One
    + Clone
    + Copy
{}

pub trait FloatLike: Arithmetic {}

impl FloatLike for f32 {}
impl FloatLike for f64 {}
impl FloatLike for Complex32 {}
impl FloatLike for Complex64 {}


pub trait TrigCore: FloatLike {
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn tan(self) -> Self;
    fn asin(self) -> Self;
    fn acos(self) -> Self;
    fn atan(self) -> Self;
}

macro_rules! impl_trig_core {
    ($t:ty) => {
        impl TrigCore for $t {
            fn sin(self) -> Self { self.sin() }
            fn cos(self) -> Self { self.cos() }
            fn tan(self) -> Self { self.tan() }
            fn asin(self) -> Self { self.asin() }
            fn acos(self) -> Self { self.acos() }
            fn atan(self) -> Self { self.atan() }
        }
    };
}

impl_trig_core!(f32);
impl_trig_core!(f64);
impl_trig_core!(Complex32);
impl_trig_core!(Complex64);


pub trait Trig: TrigCore {
    fn cos_sin(self) -> (Self, Self);
    fn sinc(self) -> Self;
}


impl Trig for f32 {
    fn cos_sin(self) -> (Self, Self) {
        let (a, b) = self.sin_cos();
        (b, a)
    }

    fn sinc(self) -> Self {
        if self == Self::zero() {
            Self::one()
        } else {
            let t = self * std::f32::consts::PI;
            t.sin() / t
        }
    }
}


impl Trig for f64 {
    fn cos_sin(self) -> (Self, Self) {
        let (a, b) = self.sin_cos();
        (b, a)
    }
    fn sinc(self) -> Self {
        if self == Self::zero() {
            Self::one()
        } else {
            let t = self * std::f64::consts::PI;
            t.sin() / t
        }
    }
}


impl Trig for Complex32 {
    fn cos_sin(self) -> (Self, Self) {
        (self.cos(), self.sin())
    }
    fn sinc(self) -> Self {
        if self == Self::zero() {
            Self::one()
        } else {
            let t = self * std::f32::consts::PI;
            t.sin() / t
        }
    }
}


impl Trig for Complex64 {
    fn cos_sin(self) -> (Self, Self) {
        (self.cos(), self.sin())
    }
    fn sinc(self) -> Self {
        if self == Self::zero() {
            Self::one()
        } else {
            let t = self * std::f64::consts::PI;
            t.sin() / t
        }
    }
}
