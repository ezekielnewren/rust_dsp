use std::error::Error;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::marker::PhantomData;
use std::ops::{AddAssign, Mul};
use std::path::PathBuf;
use alsa::PCM;
use alsa::pcm::{Access, Format, HwParams};
use hound::{WavSpec, WavWriter};
use num_complex::Complex32;
use num_traits::Zero;
use crate::traits::*;


#[derive(Default)]
pub struct BufferBank<T> {
    buff0: Vec<T>,
    buff1: Vec<T>,
    direction: bool,
}


impl<T> BufferBank<T> {
    pub fn swap(&mut self) -> (&mut Vec<T>, &mut Vec<T>) {
        let result = if self.direction {
            (&mut self.buff0, &mut self.buff1)
        } else {
            (&mut self.buff1, &mut self.buff0)
        };
        self.direction = !self.direction;
        result
    }
}


pub struct WavSink<D: Write + Seek> {
    writer: WavWriter<D>,
}


impl<D: Write + Seek> Drop for WavSink<D>  {
    fn drop(&mut self) {
        self.writer.flush().unwrap();
    }
}


impl<D: Write + Seek> WavSink<D> {
    pub fn new(sample_rate: usize, channels: u16, sink: D) -> Result<Self, Box<dyn Error>> {
        let spec = WavSpec {
            channels,
            sample_rate: sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        Ok(Self {
            writer: WavWriter::new(sink, spec)?,
        })
    }
}


impl WavSink<BufWriter<File>> {
    pub fn new_file(sample_rate: usize, channels: u16, path: PathBuf) -> Result<WavSink<BufWriter<File>>, Box<dyn Error>> {
        let spec = WavSpec {
            channels,
            sample_rate: sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        Ok(WavSink {
            writer: WavWriter::create(path, spec)?,
        })
    }
}


impl<D: Write + Seek> Sink<f32> for WavSink<D> {
    fn write(&mut self, src: &[f32]) -> Result<(), Box<dyn Error>> {
        debug_assert!(self.writer.spec().channels == 1);
        for &sample in src {
            self.writer.write_sample(sample as i16)?;
        }
        Ok(())
    }
}


impl<D: Write + Seek> Sink<Complex32> for WavSink<D> {
    fn write(&mut self, src: &[Complex32]) -> Result<(), Box<dyn Error>> {
        debug_assert!(self.writer.spec().channels == 2);
        for &sample in src {
            self.writer.write_sample(sample.re as i16)?;
            self.writer.write_sample(sample.im as i16)?;
        }
        Ok(())
    }
}


pub struct AlsaSource {
    pcm: PCM,
    samples_per_buffer: usize,
}

impl AlsaSource {
    pub fn new(sample_rate: usize, samples_per_buffer: usize, device: &str) -> Result<Self, Box<dyn Error>> {
        let it = Self {
            pcm: PCM::new(device, alsa::Direction::Capture, false)?,
            samples_per_buffer,
        };

        let hwp = HwParams::any(&it.pcm)?;

        let channels = 1;
        let format = Format::s16();

        hwp.set_channels(channels)?;
        hwp.set_rate(sample_rate as u32, alsa::ValueOr::Nearest)?;
        hwp.set_format(format)?;
        hwp.set_access(Access::RWInterleaved)?;
        it.pcm.hw_params(&hwp)?;
        drop(hwp);

        Ok(it)
    }
    
    pub fn default_source(sample_rate: usize) -> Result<Self, Box<dyn Error>> {
        Self::new(sample_rate, 1024, "default")
    }
}


impl Source<i16> for AlsaSource {
    fn read(&mut self, dst: &mut Vec<i16>) -> Result<usize, Box<dyn Error>> {
        if dst.len() != self.samples_per_buffer {
            dst.resize(self.samples_per_buffer, 0);
        }
        Ok(self.pcm.io_i16()?.readi(dst)?)
    }
}


pub struct AlsaSink {
    pcm: PCM,
}

impl AlsaSink {
    pub fn new(sample_rate: usize, device: &str) -> Result<Self, Box<dyn Error>> {
        let pcm = PCM::new(device, alsa::Direction::Playback, false)?;

        let hwp = HwParams::any(&pcm)?;
        let channels = 1;
        let format = Format::s16();

        hwp.set_channels(channels)?;
        hwp.set_rate(sample_rate as u32, alsa::ValueOr::Nearest)?;
        hwp.set_format(format)?;
        hwp.set_access(Access::RWInterleaved)?;
        pcm.hw_params(&hwp)?;
        drop(hwp);

        Ok(Self {
            pcm,
        })
    }

    pub fn default_sink(sample_rate: usize) -> Result<Self, Box<dyn Error>> {
        Self::new(sample_rate, "default")
    }
}

impl Sink<i16> for AlsaSink {
    fn write(&mut self, src: &[i16]) -> Result<(), Box<dyn Error>> {
        self.pcm.io_i16()?.writei(src)?;
        Ok(())
    }
}



pub struct MixerFilter {
    phase: f32,
    omega: f32,
}


impl MixerFilter {
    pub fn new(sample_rate: usize, freq: f32) -> Self {
        Self {
            phase: 0.0,
            omega: 2.0 * PI * freq / sample_rate as f32,
        }
    }
}

impl Filter<f32, Complex32> for MixerFilter {
    fn filter(&mut self, input: &[f32], output: &mut Vec<Complex32>) -> Result<(), Box<dyn Error>> {
        output.clear();
        for sample in input.iter().copied() {
            let (sin, cos) = self.phase.sin_cos();
            let (i, q) = (sample * cos, sample * sin);
            output.push(Complex32 {re: i, im: -q});
            self.phase = (self.phase + self.omega).rem_euclid(2.0 * PI);
        }

        Ok(())
    }
}


impl Filter<Complex32, f32> for MixerFilter {
    fn filter(&mut self, input: &[Complex32], output: &mut Vec<f32>) -> Result<(), Box<dyn Error>> {
        output.clear();
        for sample in input.iter() {
            let (sin, cos) = self.phase.sin_cos();
            let real = sample.re * cos - sample.im * sin;
            output.push(real);
            self.phase = (self.phase + self.omega).rem_euclid(2.0 * PI);
        }

        Ok(())
    }
}


pub fn cast_all<F, I, O>(func: F, input: &[I], output: &mut Vec<O>)
where F: Fn(I) -> O, I: Copy
{
    output.clear();
    for v in input.iter() {
        output.push(func(*v));
    }
}


pub struct FIRFilter<T>
where T: Arithmetic
{
    taps: Vec<T>,
    history: Vec<T>,
    index: usize,
}


impl<T: Arithmetic> FIRFilter<T> {
    pub fn new(taps: Vec<T>) -> Self {
        let len = taps.len();
        Self {
            taps,
            history: vec![T::zero(); len],
            index: 0,
        }
    }
}


impl<T: Arithmetic> Filter<T, T> for FIRFilter<T> {
    fn filter(&mut self, input: &[T], output: &mut Vec<T>) -> Result<(), Box<dyn Error>> {
        output.clear();

        for sample in input.iter().copied() {
            self.history[self.index] = sample;

            let mut acc = T::zero();
            for i in 0..self.taps.len() {
                let slot = (self.index + self.taps.len() - i) % self.taps.len();
                acc += self.taps[i] * self.history[slot];
            }
            output.push(acc);

            self.index = (self.index + 1) % self.taps.len();
        }

        Ok(())
    }
}


pub fn lowpass_real(sample_rate: usize, cutoff_hz: f32, num_taps: usize) -> FIRFilter<f32> {
    let fc = cutoff_hz / sample_rate as f32;
    let m = num_taps as isize - 1;

    let mut taps: Vec<f32> = Vec::with_capacity(num_taps);

    for n in 0..num_taps {
        let n = n as isize;
        let centered = n - m / 2;

        let sinc = if centered == 0 {
            2.0 * fc
        } else {
            (2.0 * PI *fc * centered as f32).sin() / (PI * centered as f32)
        };


        // Apply a Hamming window
        let window = 0.54 - 0.46 * ((2.0 * PI * n as f32) / m as f32 ).cos();
        taps.push(sinc * window);
    }

    FIRFilter::new(taps)
}


pub fn lowpass_complex(sample_rate: usize, cutoff_hz: f32, num_taps: usize) -> FIRFilter<Complex32> {
    let tmp = lowpass_real(sample_rate, cutoff_hz, num_taps);
    FIRFilter::new(tmp.taps.into_iter().map(|real| Complex32::from(real)).collect())
}




#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;
    use crate::traits::{Sink, Source};
    use crate::block::{cast_all, AlsaSource, WavSink};

    #[test]
    fn test_microphone() -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = 44100;

        let file_dest = PathBuf::from("/tmp/alsa.wav");

        let mut source = AlsaSource::default_source(sample_rate)?;
        let mut sink = WavSink::new_file(sample_rate, 1, file_dest)?;

        let mut total = 0;
        let mut buff_raw = Vec::new();
        let mut buff_real = Vec::new();

        let start = Instant::now();
        loop {
            if start.elapsed().as_secs_f32() > 3.0 {
                break;
            }
            if let Ok(read) = source.read(&mut buff_raw) {
                cast_all(|v| v as f32, buff_raw.as_slice(), &mut buff_real);
                sink.write(buff_real.as_slice())?;
                total += read;
            }
        }

        Ok(())
    }

}
