

pub trait Source<I> {
    fn read(&mut self, dst: &mut Vec<I>);
}

pub trait Filter<I, O> {
    fn filter(&mut self, input: &[I], output: &mut Vec<O>);
}

pub trait Sink<O> {
    fn write(&mut self, src: &[O]);
}

#[cfg(test)]
mod tests {
    use alsa::pcm::{Access, Format, HwParams, PCM};
    use hound::{WavWriter, WavSpec};

    #[test]
    fn test_microphone() -> Result<(), Box<dyn std::error::Error>> {
        // === ALSA CONFIGURATION ===
        let pcm = PCM::new("default", alsa::Direction::Capture, false)?; // "default" device
        let hwp = HwParams::any(&pcm)?;

        let sample_rate = 44100u32;
        let channels = 1;
        let format = Format::s16(); // signed 16-bit

        hwp.set_channels(channels)?;
        hwp.set_rate(sample_rate, alsa::ValueOr::Nearest)?;
        hwp.set_format(format)?;
        hwp.set_access(Access::RWInterleaved)?;
        pcm.hw_params(&hwp)?;

        // Buffer size in frames
        let io = pcm.io_i16()?; // i16 because we use Format::s16()
        let frames_per_buffer = 1024usize;
        let mut buffer = vec![0i16; frames_per_buffer * channels as usize];

        // === WAV FILE SETUP ===
        let spec = WavSpec {
            channels: channels as u16,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create("recorded.wav", spec)?;

        println!("Recording for 5 seconds...");

        let num_loops = (sample_rate as usize * 5) / frames_per_buffer; // 5 seconds of audio

        for _ in 0..num_loops {
            io.readi(&mut buffer)?;
            for sample in &buffer {
                writer.write_sample(*sample)?;
            }
        }

        println!("Done! Saved as recorded.wav");
        writer.finalize()?;
        Ok(())
    }

}