use std::error::Error;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;
use bitvec::prelude::*;
use num_complex::Complex32;
use crate::traits::{Filter, Sink, Source};
use crate::block::*;

pub mod traits;
pub mod block;
pub mod ringbuf;
pub mod streambuf;
pub mod util;

struct Tone {
    freq: f32,
    amp: f32,
}


struct BitStream<'a, IT>
where IT: Iterator<Item = &'a u8>
{
    it: IT,
    byte: u8,
    i: usize,
}

impl<'a, IT> BitStream<'a, IT>
where IT: Iterator<Item = &'a u8>
{

    fn new(it: IT) -> Self {
        Self {
            it,
            byte: 0,
            i: 0,
        }
    }

}

impl<'a, IT> Iterator for BitStream<'a, IT>
where IT: Iterator<Item = &'a u8>
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let (q, r) = (self.i >> 3, self.i & 0x7);
        if r == 0 {
            if let Some(v) = self.it.next() {
                self.byte = *v;
            } else {
                return None;
            }
        }

        let bit = (self.byte >> r) & 1;
        self.i += 1;
        Some(bit)
    }
}


fn canonical_path(path: String) -> PathBuf {
    if path.starts_with("~/") {
        dirs::home_dir().unwrap().join(&path.as_str()[2..])
    } else {
        PathBuf::from(path)
    }
}



fn main() -> Result<(), Box<dyn Error>> {
    let dir_dump = dirs::home_dir().unwrap().join("tmp");

    let argv: Vec<String> = std::env::args().collect();

    let file_src = canonical_path(argv[1].clone());
    
    let mut source = WavSource::new(file_src, 1024)?;
    let sample_rate = source.spec().sample_rate as usize;
    let mut sink = Speakers::new(sample_rate, 2)?;
    
    let mut raw = Vec::<Complex32>::new();
    let mut out = Vec::<i16>::new();
    
    while let Ok(()) = source.read(&mut raw) {
        if raw.len() == 0 {
            break;
        }
        out.clear();
        for v in raw.iter().copied() {
            out.push(v.re as i16);
            out.push(v.im as i16);
        }
        sink.write(out.as_slice())?;
    }
    drop(sink);
    
    Ok(())
}

