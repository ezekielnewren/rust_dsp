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
    let args = std::env::args().nth(1).ok_or("missing tune frequency")?;
    let tune_freq: f32 = args.as_str().parse()?;
    
    let device = HackRf::open()?;
    
    let cutoff_hz = 200e3f32;
    let tune_off = -2.0 * cutoff_hz;
    let tune_hardware = (tune_freq + tune_off) as u64;

    let bandwidth: u32 = 2_000_000;
    let sample_rate_hardware: u32 = bandwidth * 2;
    let sample_rate_fm = (2.0 * cutoff_hz) as u32;
    let sample_rate_audio: u32 = 44100;
    
    device.set_sample_rate(sample_rate_hardware)?;
    device.set_baseband_filter_bandwidth(bandwidth)?;
    device.set_freq(tune_hardware)?;
    device.set_amp_enable(false)?;
    
    // device.set_lna_gain(0)?;
    // device.set_rxvga_gain(0)?;

    let mut bank_complex = BufferBank::default();
    let mut bank_real = BufferBank::<f32>::default();

    let mut source = HackRFSource::new(device, sample_rate_hardware as usize)?;
    let mut mix = MixerFilter::new(sample_rate_hardware, tune_off);
    let mut resample0 = RationalResampler::new(sample_rate_hardware, sample_rate_fm, 101);
    let mut demod = FMDemod::new(sample_rate_fm, 75e3);
    let mut resample1 = RationalResampler::new(sample_rate_fm, sample_rate_audio, 101);
    let mut deemph = DeEmphasisFilter::new(sample_rate_audio, 75e-6);
    let mut sink = Speakers::new(sample_rate_audio, 1)?;
    
    let mut total: u64 = 0;
    
    let mut frame = 0;
    
    let start = Instant::now();
    loop {
        let (src, dst) = bank_complex.swap();
        if let Ok(()) = source.read(src) {
            println!("{}: new frame", frame);
            total += src.len() as u64;
            if src.len() == 0 || start.elapsed().as_secs_f32() > u64::MAX as f32 {
                break;
            }

            mix.filter(src, dst)?;
            println!("{}: mix", frame);
            
            let (src, dst) = bank_complex.swap();
            resample0.filter(src, dst)?;
            println!("{}: extract fm band", frame);

            let (src, _) = bank_complex.swap();
            let (_, dst) = bank_real.swap();
            demod.filter(src, dst)?;
            println!("{}: fm demod", frame);
            
            let (src, dst) = bank_real.swap();
            resample1.filter(src, dst)?;
            println!("{}: audio downsample, {}", frame, dst.len());

            let (src, dst) = bank_real.swap();
            deemph.filter(src, dst)?;
            println!("{}: de-emphasis, {}", frame, dst.len());
            
            sink.write(dst.as_slice())?;
            println!("{}: done", frame);
            
            frame += 1;
        }
    }

    println!("samples captured: {}", total);

    Ok(())
}

