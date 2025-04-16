use std::f32::consts::PI;
use std::fs::File;
use std::io::Read;
use bitvec::prelude::*;

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
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let (q, r) = (self.i >> 3, self.i & 0x7);
        if r == 0 {
            if let Some(v) = self.it.next() {
                self.byte = *v;
            } else {
                return None;
            }
        }

        let bit = ((self.byte >> r) & 1) as f32;
        self.i += 1;
        Some(bit)
    }
}



fn main() {
    let dir_dump = dirs::home_dir().unwrap().join("tmp");

    let argv: Vec<String> = std::env::args().collect();

    let mut file_message = File::open(&argv[1]).unwrap();
    let mut buffer = Vec::new();
    file_message.read_to_end(&mut buffer).unwrap();
    drop(file_message);

    let sample_rate = 44100.0;
    let baud = 10.0f32;
    let carrier = Tone { freq: 1000.0, amp: 0.5 };
    let deviation = 500.0;

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 8,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(dir_dump.join("tmp.wav"), spec).unwrap();

    let mut phase = 0.0f32;

    let samples_per_symbol = 1.0 / baud * sample_rate;
    for bit in BitStream::new(buffer.as_slice().iter()) {
        let bit = bit * 2.0 - 1.0;
        let inst_freq = carrier.freq + bit * deviation;

        for _ in 0..samples_per_symbol as usize {
            let sample = carrier.amp * phase.cos();
            phase += 2.0 * PI * inst_freq / sample_rate;

            let int_samp = (sample * i8::MAX as f32) as i8;
            writer.write_sample(int_samp).unwrap();
        }

        phase %= 2.0 * PI;
    }

    writer.finalize().unwrap();

    println!("{}", dir_dump.as_path().display());

}

