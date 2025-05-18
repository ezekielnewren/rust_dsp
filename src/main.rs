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
    let mut sink = AlsaSink::new(sample_rate, source.spec().channels as u32, "default")?;
    
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
    
    // let sample_rate: usize = 44100;
    // let carrier_freq = 1350.0;
    // let cutoff_hz = 1500f32.max(carrier_freq);
    // 
    // let mut mixer = MixerFilter::new(sample_rate, carrier_freq);
    // let mut lpf = lowpass_complex(sample_rate, cutoff_hz, 101);
    // 
    // let file_dest = canonical_path(argv[1].clone());
    // 
    // let mut source = AlsaSource::default_source(sample_rate)?;
    // let mut sink = WavSink::new_file(sample_rate, 1, file_dest)?;
    // 
    // let mut total = 0;
    // let mut buff_raw_samples = Vec::<i16>::new();
    // let mut buff_real_samples = Vec::<f32>::new();
    // let mut bank = BufferBank::<Complex32>::default();
    // 
    // let start = Instant::now();
    // loop {
    //     if start.elapsed().as_secs_f32() > 5.0 {
    //         break;
    //     }
    //     if let Ok(read) = source.read(&mut buff_raw_samples) {
    //         cast_all(|v| v as f32, buff_raw_samples.as_slice(), &mut buff_real_samples);
    //         
    //         let (_, dst) = bank.swap();
    //         mixer.filter(buff_real_samples.as_slice(), dst)?;
    //         
    //         let (src, dst) = bank.swap();
    //         lpf.filter(src.as_slice(), dst)?;
    //         
    //         // do something with the IQ samples
    //         
    //         
    //         // write the IQ samples back out
    //         let (src, _) = bank.swap();
    //         mixer.filter(src.as_slice(), &mut buff_real_samples)?;
    //         total += buff_real_samples.len();
    //         sink.write(buff_real_samples.as_slice())?;
    //     }
    // }
    // println!("Total: {}", total);

    Ok(())
}

