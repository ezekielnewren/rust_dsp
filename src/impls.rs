use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::PathBuf;
use alsa::PCM;
use alsa::pcm::{Access, Format, HwParams};
use hound::{WavSpec, WavWriter};
use crate::block::*;


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


impl<D: Write + Seek> Sink<i16> for WavSink<D> {
    fn write(&mut self, src: &[i16]) -> Result<(), Box<dyn Error>> {
        for &sample in src {
            self.writer.write_sample(sample)?;
        }
        Ok(())
    }
}


pub struct AlsaSource {
    pcm: PCM,
}

impl AlsaSource {
    pub fn new(sample_rate: usize, device: &str) -> Result<Self, Box<dyn Error>> {
        let it = Self {
            pcm: PCM::new(device, alsa::Direction::Capture, false)?,
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
        Self::new(sample_rate, "default")
    }
}


impl Source<i16> for AlsaSource {
    fn read(&mut self, dst: &mut [i16]) -> Result<usize, Box<dyn Error>> {
        Ok(self.pcm.io_i16()?.readi(dst)?)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;
    use crate::block::{Sink, Source};
    use crate::impls::{AlsaSource, WavSink};

    #[test]
    fn test_microphone() -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = 44100;

        let file_dest = PathBuf::from("/tmp/alsa.wav");

        let mut source = AlsaSource::default_source(sample_rate)?;
        let mut sink = WavSink::new_file(sample_rate, 1, file_dest)?;

        let mut total = 0;
        let mut buff = vec![0; 1024];

        let start = Instant::now();
        loop {
            if start.elapsed().as_secs_f32() > 3.0 {
                break;
            }
            if let Ok(read) = source.read(buff.as_mut_slice()) {
                sink.write(buff.as_slice())?;
                total += read;
            }
        }

        Ok(())
    }

}
