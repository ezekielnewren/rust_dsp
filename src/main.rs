use std::error::Error;
use std::f32::consts::PI;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;
use bitvec::prelude::*;
use crate::block::{Sink, Source};
use crate::impls::{AlsaSource, WavSink};

pub mod block;
mod impls;

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

    let sample_rate: usize = 44100;

    let file_dest = canonical_path(argv[1].clone());
    
    let mut source = AlsaSource::default_source(sample_rate)?;
    let mut sink = WavSink::new_file(sample_rate, 1, file_dest)?;

    let mut total = 0;
    let mut buff0 = vec![0; 1024];
    
    let start = Instant::now();
    loop {
        if start.elapsed().as_secs_f32() > 3.0 {
            break;
        }
        if let Ok(read) = source.read(buff0.as_mut_slice()) {
            sink.write(buff0.as_slice())?;
            total += read;
        }
    }
    
    println!("Total: {}", total);
    
    Ok(())
}

