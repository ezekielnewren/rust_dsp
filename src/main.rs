use std::f32::consts::PI;
use std::fs::File;
use std::io::Read;
use bitvec::prelude::*;

struct Tone {
    freq: f32,
    amp: f32,
}


fn main() {
    let dir_dump = dirs::home_dir().unwrap().join("tmp");

    let argv: Vec<String> = std::env::args().collect();

    let mut file_message = File::open(&argv[1]).unwrap();
    let mut buffer = Vec::new();
    file_message.read_to_end(&mut buffer).unwrap();
    drop(file_message);

    let sample_rate = 44100.0;
    let sps = 0.01;

    let lo = Tone { freq: 440.0, amp: 1.0 };
    let hi = Tone { freq: 660.0, amp: 1.0 };

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 8,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(dir_dump.join("tmp.wav"), spec).unwrap();

    let mut bitstream = bitvec![0; 0];
    for b in buffer.as_slice() {
        bitstream.push(((*b >> 0) & 1) != 0);
        bitstream.push(((*b >> 1) & 1) != 0);
        bitstream.push(((*b >> 2) & 1) != 0);
        bitstream.push(((*b >> 3) & 1) != 0);
        bitstream.push(((*b >> 4) & 1) != 0);
        bitstream.push(((*b >> 5) & 1) != 0);
        bitstream.push(((*b >> 6) & 1) != 0);
        bitstream.push(((*b >> 7) & 1) != 0);
    }

    for bit in bitstream {
        let tone = if bit { &hi } else { &lo };
        let mut tick = 0.0f32;
        for _ in 0..(sps * sample_rate) as usize {
            let sample = tone.amp * (2.0 * PI * tone.freq * (tick / sample_rate)).sin();
            let int_samp = (sample * i8::MAX as f32) as i8;
            writer.write_sample(int_samp).unwrap();
            tick += 1.0;
        }
    }

    writer.finalize().unwrap();

    println!("{}", dir_dump.as_path().display());

}

