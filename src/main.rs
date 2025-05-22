use std::error::Error;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;
use bitvec::prelude::*;
use libhackrf::ffi::HackrfDevice;
use libhackrf::HackRf;
use num_complex::Complex32;
use crate::traits::{Filter, Sink, Source};
use crate::block::*;

pub mod traits;
pub mod block;
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
    
    // let mut source = WavSource::new(file_src, 0)?;
    // let sample_rate = source.spec().sample_rate as usize;
    // let mut sink = Speakers::new(sample_rate, 2)?;
    
    let device = HackRf::open()?;
    
    let tune_freq = 95.5e6 as u64;
    let tune_off = 200e3f32 as u64;
    let tune_hardware = tune_freq - tune_off;
    
    let sample_rate = 2_000_000;
    
    device.set_sample_rate(sample_rate)?;
    device.set_freq(tune_hardware)?;
    device.set_amp_enable(false)?;
    
    let mut source = HackRFSource::new(device, sample_rate as usize)?;
    
    
    let mut raw = Vec::<Complex32>::new();
    
    let mut total: u64 = 0;
    
    let start = Instant::now();
    while let Ok(()) = source.read(&mut raw) {
        if raw.len() == 0 || start.elapsed().as_secs_f32() > 3.0 {
            break;
        }
        total += raw.len() as u64;
        println!("samples: {} {}", raw.len(), total);
        // out.clear();
        // for v in raw.iter().copied() {
        //     out.push(v.re);
        //     out.push(v.im);
        // }
        // sink.write(out.as_slice())?;
    }
    // drop(sink);
    
    println!("samples captured: {}", total);
    
    Ok(())
}

