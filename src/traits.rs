use std::error::Error;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use num_traits::{One, Zero};

pub trait Source<I> {
    fn read(&mut self, dst: &mut Vec<I>) -> Result<usize, Box<dyn Error>>;
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
    + Zero
    + One
    + Clone
    + Copy
{}
