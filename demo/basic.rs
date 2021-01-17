const SAMPLE_RATE: i128 = 44100;
const CLOCK_RATE: i128 = 3579545;

use blipbuff::blipbuffer::BlipBuffer;
use unwrap::unwrap;

pub struct Demo1 {
    time: i128,
    period: i128,
    phase: i128,
    volume: i128,
    amplitude: i128,
    blip_buffer: BlipBuffer,
}

impl Demo1 {
    pub fn run_wave(&mut self, clocks: i128) {
        while self.time < clocks {
            let delta = self.phase * self.volume - self.amplitude;
            self.amplitude = self.amplitude + delta;
            self.blip_buffer.add_delta(self.time, delta);
            self.phase = -1 * self.phase;
            self.time = self.time + self.period;
        }
    }

    pub fn end_wave(&mut self, clocks: i128) {
        self.blip_buffer.end_frame(clocks);
        self.time = self.time - clocks;
    }

    pub fn modify_wave(&mut self) {
        self.volume = self.volume + 100;
        self.period = self.period + (self.period / 28 + 3);
    }

    pub fn samples_available(&mut self) -> bool {
        return self.blip_buffer.samples_available() > 0;
    }

    pub fn read_samples(&mut self, count: i128) -> (i128, Vec<i128>) {
        self.blip_buffer.read_samples(count, false)
    }
}

pub fn run() {
    let mut demo1 = Demo1 {
        time: 0,
        period: 1,
        phase: 1,
        volume: 0,
        amplitude: 0,
        blip_buffer: BlipBuffer::new(SAMPLE_RATE / 10),
    };

    demo1.blip_buffer.set_rates(CLOCK_RATE, SAMPLE_RATE);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("basic.wav", spec).unwrap();
    let mut total_samples_written: i128 = 0;

    while total_samples_written < (SAMPLE_RATE * 2) {
        let clocks = CLOCK_RATE / 60;

        demo1.run_wave(clocks);
        demo1.end_wave(clocks);

        while demo1.samples_available() {
            let (read_count, samples) = demo1.read_samples(512);

            for sample in samples.iter() {
                //println!("Sample: {}", sample);
                unwrap!(
                    writer.write_sample(*sample as i16),
                    "Unable to write sample: {}",
                    sample
                );
            }

            total_samples_written = total_samples_written + read_count as i128;
        }

        demo1.modify_wave();
    }

    writer.finalize().unwrap();
}
