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
use crate::util::BufferBank;

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

    let file_dst = canonical_path(argv[1].clone());
    
    // let mut source = WavSource::new(file_src, 0)?;
    // let sample_rate = source.spec().sample_rate as usize;
    // let mut sink = Speakers::new(sample_rate, 2)?;

    let device = HackRf::open()?;
    
    let cutoff_hz = 200e3f32;
    
    let tune_freq = 95.5e6;
    let tune_off = -cutoff_hz;
    let tune_hardware = (tune_freq + tune_off) as u64;

    let bandwidth: u32 = 2_000_000;
    let sample_rate: u32 = bandwidth * 2;

    device.set_sample_rate(sample_rate)?;
    device.set_baseband_filter_bandwidth(bandwidth)?;
    device.set_freq(tune_hardware)?;
    device.set_amp_enable(false)?;

    let mut bank = BufferBank::default();

    let mut source = HackRFSource::new(device, sample_rate as usize)?;
    let mut mix = MixerFilter::new(sample_rate, tune_off);
    let mut resample = RationalResampler::new(sample_rate, (2.0 * cutoff_hz) as u32, 101);
    let mut sink = WavSink::new_file(sample_rate, 2, file_dst)?;

    let mut total: u64 = 0;

    let start = Instant::now();
    loop {
        let (src, dst) = bank.swap();
        if let Ok(()) = source.read(src) {
            total += src.len() as u64;
            if src.len() == 0 || start.elapsed().as_secs_f32() > 3.0 {
                break;
            }

            mix.filter(src, dst)?;
            let (src, dst) = bank.swap();
            resample.filter(src, dst)?;
            // lowpass.filter(src, dst)?;
            sink.write(dst.as_slice())?;
        }
    }

    println!("samples captured: {}", total);

    Ok(())
}

