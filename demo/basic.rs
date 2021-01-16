const SAMPLE_RATE: u64 = 44100;
const CLOCK_RATE: i64 = 3579545;

use blipbuff::blipbuffer::BlipBuffer;

pub struct Demo1 {
    time: i64,
    period: i64,
    phase: i64,
    volume: i64,
    amplitude: i64,
    blip_buffer: BlipBuffer,
}

impl Demo1 {
    pub fn run_wave(&mut self, clocks: i64) -> Vec<i128> {
        while clocks < self.time {
            let delta = self.phase * self.volume - self.amplitude;
            self.amplitude = self.amplitude + delta;
            self.blip_buffer.add_delta(self.time as u32, delta as u32);
            self.time = self.time + self.period;
        }

        println!("Time: {}", self.time);

        self.blip_buffer.end_frame(clocks as u32);
        self.time = self.time - clocks;

        self.volume = self.volume + 100;
        self.period += self.period / 28 + 3;

        return if self.blip_buffer.samples_available() > 0 {
            self.blip_buffer.read_samples(512, false)
        } else {
            Vec::new()
        }
    }
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

    let clocks = CLOCK_RATE  / 60;

    let mut total_samples_written: u64 = 0;

    while total_samples_written < SAMPLE_RATE * 2 {
        let samples = demo1.run_wave(clocks);

        for sample in samples.iter() {
            println!("Sample: {}", sample);
            writer.write_sample(*sample as i32).unwrap();
        }

        total_samples_written = total_samples_written + samples.len() as u64;
    }


    writer.finalize().unwrap();
}
