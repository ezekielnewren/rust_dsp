use std::error::Error;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Seek, Write};
use std::ops::{AddAssign, Mul};
use std::path::PathBuf;
use cpal::{BufferSize, Stream, StreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavReader, WavSpec, WavWriter};
use num_complex::Complex32;
use num_traits::Zero;
use crate::streambuf::{new_stream, StreamReader, StreamWriter};
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


pub struct WavSource<D: Read> {
    reader: WavReader<D>,
    samples_per_buffer: usize,
}


impl WavSource<BufReader<File>> {
    pub fn new(path: PathBuf, samples_per_buffer: usize) -> Result<Self, Box<dyn Error>> {
        let mut it = Self {
            reader: WavReader::open(path)?,
            samples_per_buffer,
        };
        if samples_per_buffer == 0 {
            it.samples_per_buffer = it.reader.spec().sample_rate as usize;
        }
        Ok(it)
    }

    pub fn spec(&self) -> WavSpec {
        self.reader.spec()
    }
}


impl<D: Read> Source<f32> for WavSource<D> {
    fn read(&mut self, dst: &mut Vec<f32>) -> Result<(), Box<dyn Error>> {
        debug_assert!(self.reader.spec().channels == 1);
        dst.clear();
        let it = self.reader.samples::<i16>();
        for sample in it {
            dst.push(sample? as f32);
            if dst.len() >= self.samples_per_buffer {
                break;
            }
        }
        Ok(())
    }
}


impl<D: Read> Source<Complex32> for WavSource<D> {
    fn read(&mut self, dst: &mut Vec<Complex32>) -> Result<(), Box<dyn Error>> {
        debug_assert!(self.reader.spec().channels == 2);
        dst.clear();
        let mut it = self.reader.samples::<i16>();
        while let Some(Ok(re)) = it.next() {
            if let Some(Ok(im)) = it.next() {
                let c = Complex32::new(re as f32, im as f32);
                dst.push(c);
                if dst.len() >= self.samples_per_buffer {
                    break;
                }
            } else {
                return Err(Box::new(std::io::Error::new(ErrorKind::UnexpectedEof, "unexpected eof")));
            }
        }
        Ok(())
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


pub struct CpalSource {
    audio_stream: Stream,
    config: StreamConfig,
    reader: StreamReader<i16>,
}

impl CpalSource {
    pub fn new(sample_rate: usize) -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("unable to open default input audio device")?;

        let config = StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(sample_rate as u32),
            buffer_size: BufferSize::Default,
        };

        let (reader, writer) = new_stream::<i16>(sample_rate, true, false, true)?;


        let stream = device.build_input_stream(&config, move |data: &[i16], _: &cpal::InputCallbackInfo| {
            writer.put(data).unwrap();
        },
                                               move |error: cpal::StreamError| {
                                                   panic!("{}", error);
                                               },
                                               None
        )?;

        let src = Self {
            audio_stream: stream,
            config,
            reader,
        };

        src.audio_stream.play()?;

        Ok(src)
    }
}


impl Source<i16> for CpalSource {
    fn read(&mut self, dst: &mut Vec<i16>) -> Result<(), Box<dyn Error>> {
        let sample_rate = self.config.sample_rate.0 as usize;
        if dst.capacity() < sample_rate {
            dst.reserve(sample_rate - dst.capacity());
        }
        if dst.len() < sample_rate {
            unsafe { dst.set_len(dst.capacity()); }
        }

        let read = self.reader.get(dst.as_mut_slice())?;
        unsafe { dst.set_len(read); }
        Ok(())
    }
}


pub struct CpalSink {
    audio_stream: Stream,
    config: StreamConfig,
    writer: StreamWriter<i16>,
}

impl CpalSink {
    pub fn new(sample_rate: usize, channels: u16) -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("unable to open default output audio device")?;

        let config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate as u32),
            buffer_size: BufferSize::Default,
        };

        let (reader, writer) = new_stream::<i16>(sample_rate, false, true, false)?;


        let stream = device.build_output_stream(&config, move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
            let result = reader.get(data);
            match result {
                Ok(0) => {
                    data.fill(0);
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    data.fill(0);
                },
                Ok(read) => {
                    if read < data.len() {
                        data[read..].fill(0);
                    }
                },
                Err(e) => {
                    panic!("{}", e);
                }
            }
        },
                                                move |error: cpal::StreamError| {
                                                    panic!("{}", error);
                                                },
                                                None
        )?;

        let dst = Self {
            audio_stream: stream,
            config,
            writer,
        };

        dst.audio_stream.play()?;

        Ok(dst)
    }
}


impl Sink<i16> for CpalSink {
    fn write(&mut self, src: &[i16]) -> Result<(), Box<dyn Error>> {
        let mut off = 0;
        while off < src.len() {
            off += self.writer.put(&src[off..])?;
        }
        Ok(())
    }
}


impl Drop for CpalSink {
    fn drop(&mut self) {
        self.writer.drain().unwrap()
    }
    
}


pub type Microphone = CpalSource;
pub type Speakers = CpalSink;


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
    use crate::block::{cast_all, Microphone, WavSink};

    #[test]
    fn test_microphone() -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = 44100;

        let file_dest = PathBuf::from("/tmp/cpal.wav");

        let mut source = Microphone::new(sample_rate)?;
        let mut sink = WavSink::new_file(sample_rate, 1, file_dest)?;

        let mut total = 0;
        let mut buff_raw = Vec::new();
        let mut buff_real = Vec::new();

        let start = Instant::now();
        loop {
            if start.elapsed().as_secs_f32() > 3.0 {
                break;
            }
            if let Ok(()) = source.read(&mut buff_raw) {
                cast_all(|v| v as f32, buff_raw.as_slice(), &mut buff_real);
                total += buff_real.len();
                sink.write(buff_real.as_slice())?;
            }
        }

        Ok(())
    }

}
