const SAMPLE_RATE: u64 = 44100;
const CLOCK_RATE: u64 = 3579545;

use blipbuff::blipbuffer::BlipBuffer;
use std::f32::consts::PI;
use std::i16;

pub struct Demo1 {
    time: u32,
    period: u32,
    phase: u32,
    volume: u32,
    amplitude: u32,
    blip_buffer: BlipBuffer,
}

pub fn run() {
    println!("Running demo basic");

    let mut demo1 = Demo1 {
        time: 0,
        period: 1,
        phase: 1,
        volume: 0,
        amplitude: 0,
        blip_buffer: BlipBuffer::new((SAMPLE_RATE / 10) as u32),
    };

    demo1.blip_buffer.set_rates(CLOCK_RATE, SAMPLE_RATE);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("basic.wav", spec).unwrap();
    let amplitude = i16::MAX as f32;

    for t in (0..44100).map(|x| x as f32 / 44100.0) {
        let sample = (t * 440.0 * 2.0 * PI).sin();
        writer.write_sample((sample * amplitude) as i16).unwrap();
    }

    writer.finalize().unwrap();
}
