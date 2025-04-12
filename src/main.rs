use std::f32::consts::PI;
use std::path::PathBuf;


struct Tone {
    freq: f32,
    amp: f32,
}


fn main() {
    let dir_dump = dirs::home_dir().unwrap().join("tmp");


    let sample_rate = 44100;
    let duration_secs = 2;

    let mut mixer = Vec::new();
    mixer.push(Tone { freq: 440.0, amp: 0.9 });
    mixer.push(Tone { freq: 660.0, amp: 0.5 });

    let total_samples = sample_rate * duration_secs;

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 8,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(dir_dump.join("tmp.wav"), spec).unwrap();

    for n in 0..total_samples {
        let t = n as f32 / sample_rate as f32;
        let mut sample = 0.0;
        for v in mixer.iter() {
            sample += v.amp * (2.0 * PI * v.freq * t).sin();
        }
        sample /= mixer.len() as f32;

        let int_samp = (sample * i8::MAX as f32) as i8;
        writer.write_sample(int_samp).unwrap();
    }

    writer.finalize().unwrap();

    println!("{}", dir_dump.as_path().display());

}

