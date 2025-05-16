

pub trait Source<I> {
    fn read(&mut self, dst: &mut Vec<I>);
}

pub trait Filter<I, O> {
    fn filter(&mut self, input: &[I], output: &mut Vec<O>);
}

pub trait Sink<O> {
    fn write(&mut self, src: &[O]);
}

