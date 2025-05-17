use std::error::Error;

pub trait Source<I> {
    fn read(&mut self, dst: &mut [I]) -> Result<usize, Box<dyn Error>>;
}

pub trait Filter<I, O> {
    fn filter(&mut self, input: &[I], output: &mut Vec<O>) -> Result<(), Box<dyn Error>>;
}

pub trait Sink<O> {
    fn write(&mut self, src: &[O]) -> Result<(), Box<dyn Error>>;
}

