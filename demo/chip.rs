const SAMPLE_RATE: i128 = 44100;
const CLOCK_RATE: i128 = 1789772;

const MASTER_VOLUME: i128 = 65536 / 15;

use blipbuff::blipbuffer::BlipBuffer;
use unwrap::unwrap;
use std::collections::HashMap;
use chip::ChannelType::{Square, Triangle, Noise};
use text_io::scan;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;


pub enum ChannelType {
    Square,
    Triangle,
    Noise
}

struct ChannelRegister {
    period: i128,
    volume: i128,
    timbre: i128
}

pub struct Channel {
    channel_type: ChannelType,
    gain: i128,
    registers: ChannelRegister,
    time: i128,
    phase: i128,
    amplitude: i128
}

pub struct DemoChip {
    channels: HashMap<u8, Channel>,
    blip_buffer: BlipBuffer,
}

impl DemoChip {
    pub fn new() -> DemoChip {
        let mut channels = HashMap::new();

        channels.insert(0, Channel::new(Square, 26, 10));
        channels.insert(1, Channel::new(Square, 26, 10));
        channels.insert(2, Channel::new(Triangle, 30, 10));
        channels.insert(3, Channel::new(Noise, 18, 10));

        return DemoChip {
            channels,
            blip_buffer: BlipBuffer::new(SAMPLE_RATE / 10)
        }
    }

    pub fn channel_count(&mut self) -> u8 {
        return self.channels.len() as u8;
    }

    pub fn run_channel(&mut self, channel_id: u8, end_time: i128, address: i128, data: i128) {
        let channel = self.channels.get_mut(&channel_id).unwrap();

        channel.run(&mut self.blip_buffer, end_time);
        channel.update_register(address as i8, data);
    }

    pub fn close_channels(&mut self, end_time: i128) {
        for (_, channel) in self.channels.iter_mut() {
            channel.run(&mut self.blip_buffer, end_time);
            channel.close(end_time);
        }

        self.blip_buffer.end_frame(end_time as i128);
    }

    pub fn samples_available(&mut self) -> bool {
        return self.blip_buffer.samples_available() > 0;
    }

    pub fn read_samples(&mut self, count: i128) -> (i128, Vec<i128>) {
        self.blip_buffer.read_samples(count, false)
    }
}

impl Channel {
    pub fn new(channel_type: ChannelType, gain: i128, period: i128) -> Channel {
        return Channel {
            channel_type,
            gain: MASTER_VOLUME * gain / 100,
            registers: ChannelRegister {
                period,
                volume: 0,
                timbre: 0
            },
            time: 0,
            phase: 0,
            amplitude: 0
        }
    }

    pub fn run(&mut self, blip_buffer: &mut BlipBuffer, end_time: i128) {
        match self.channel_type {
            Square => self.run_square(blip_buffer, end_time),
            Triangle => self.run_triangle(blip_buffer, end_time),
            Noise => self.run_noise(blip_buffer, end_time),
        }
    }

    pub fn update_register(&mut self, register_index: i8, data: i128) {
        match register_index {
            0 => self.registers.period = data,
            1 => self.registers.volume = data,
            2 => self.registers.timbre = data,
            _ => panic!("Invalid register index")
        }
    }

    fn run_square(&mut self, blip_buffer: &mut BlipBuffer, end_time: i128) {
        while self.time < end_time {
            self.time += self.registers.period;
            self.phase = (self.phase + 1) % 8;
            let delta = self.update_amplitude(if self.phase < self.registers.timbre { 0 } else { self.registers.volume});
            blip_buffer.add_delta(self.time, delta);
        }
    }

    fn run_noise(&mut self, blip_buffer: &mut BlipBuffer, end_time: i128) {
        if self.phase == 0 {
            self.phase = 1;
        }

        while self.time < end_time {
            self.time += self.registers.period;
            self.phase = ((self.phase & 1) * self.registers.timbre) ^ (self.phase>>1);
            let delta = self.update_amplitude((self.phase & 1) * self.registers.volume);
            blip_buffer.add_delta(self.time, delta);
        }
    }

    fn run_triangle(&mut self, blip_buffer: &mut BlipBuffer, end_time: i128) {
        while self.time < end_time {
            self.time += self.registers.period;

            if self.registers.volume != 0 {
                self.phase = (self.phase + 1) % 32;
                let delta = self.update_amplitude(if self.phase < 16 { self.phase } else { 31 - self.phase });
                blip_buffer.add_delta(self.time, delta);
            }
        }
    }

    fn update_amplitude(&mut self, amplitude: i128) -> i128 {
        let delta = amplitude * self.gain - self.amplitude;
        self.amplitude += delta;
        return delta;
    }

    pub fn close(&mut self, end_time: i128) {
        self.time -= end_time;
    }
}

pub fn run() {
    let mut demo_chip = DemoChip::new();
    demo_chip.blip_buffer.set_rates(CLOCK_RATE, SAMPLE_RATE);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("chip.wav", spec).unwrap();
    let mut total_samples_written: i128 = 0;

    let mut lines = read_lines("demo/demo_log.txt").unwrap();

    while total_samples_written < (SAMPLE_RATE * 120) {

        let line = lines.next();

        if line.is_none() {
            break;
        }

        let actual_line = line.unwrap().unwrap();

        let (time, channel, address, data): (i128, u8, i128, i128);
        scan!(actual_line.bytes() => "{} {} {} {}", time, channel, address, data);

        if channel < demo_chip.channel_count() {
            demo_chip.run_channel(channel, time, address, data)
        } else {
            demo_chip.close_channels(time);
        }

        while demo_chip.samples_available() {
            let (read_count, samples) = demo_chip.read_samples(1024);

            for sample in samples.iter() {
                unwrap!(
                    writer.write_sample(*sample as i16),
                    "Unable to write sample: {}",
                    sample
                );
            }

            total_samples_written = total_samples_written + read_count as i128;
        }
    }

    writer.finalize().unwrap();
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}