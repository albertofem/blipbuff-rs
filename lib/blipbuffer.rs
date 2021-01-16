use std::num::Wrapping;
use std::collections::VecDeque;

const PRE_SHIFT: u16 = 32;
const TIME_BITS: u16 = PRE_SHIFT + 20;

const BASS_SHIFT: u16 = 9;
const END_FRAME_EXTRA: u16 = 2;

const HALD_WIDTH: u16 = 8;
const BUFFER_EXTRA: u16 = HALD_WIDTH * 2 + END_FRAME_EXTRA;
const PHASE_BITS: u16 = 5;
const PHASE_COUNT: u32 = 1 << PHASE_BITS;
const DELTA_BITS: u16 = 15;
const DELTA_UNITS: u16 = 1 << DELTA_BITS;
const FRAC_BITS: u16 = TIME_BITS - PRE_SHIFT;

const TIME_UNIT: u64 = 1 << TIME_BITS;
const BLIP_MAX_RATIO: u64 = 1 << 20;

const MAX_SAMPLE: i32 = 32767;

const BL_STEP: [[i32; 8]; 33] = [
    [   43, -115,  350, -488, 1136, -914, 5861,21022],
    [   44, -118,  348, -473, 1076, -799, 5274,21001],
    [   45, -121,  344, -454, 1011, -677, 4706,20936],
    [   46, -122,  336, -431,  942, -549, 4156,20829],
    [   47, -123,  327, -404,  868, -418, 3629,20679],
    [   47, -122,  316, -375,  792, -285, 3124,20488],
    [   47, -120,  303, -344,  714, -151, 2644,20256],
    [   46, -117,  289, -310,  634,  -17, 2188,19985],
    [   46, -114,  273, -275,  553,  117, 1758,19675],
    [   44, -108,  255, -237,  471,  247, 1356,19327],
    [   43, -103,  237, -199,  390,  373,  981,18944],
    [   42,  -98,  218, -160,  310,  495,  633,18527],
    [   40,  -91,  198, -121,  231,  611,  314,18078],
    [   38,  -84,  178,  -81,  153,  722,   22,17599],
    [   36,  -76,  157,  -43,   80,  824, -241,17092],
    [   34,  -68,  135,   -3,    8,  919, -476,16558],
    [   32,  -61,  115,   34,  -60, 1006, -683,16001],
    [   29,  -52,   94,   70, -123, 1083, -862,15422],
    [   27,  -44,   73,  106, -184, 1152,-1015,14824],
    [   25,  -36,   53,  139, -239, 1211,-1142,14210],
    [   22,  -27,   34,  170, -290, 1261,-1244,13582],
    [   20,  -20,   16,  199, -335, 1301,-1322,12942],
    [   18,  -12,   -3,  226, -375, 1331,-1376,12293],
    [   15,   -4,  -19,  250, -410, 1351,-1408,11638],
    [   13,    3,  -35,  272, -439, 1361,-1419,10979],
    [   11,    9,  -49,  292, -464, 1362,-1410,10319],
    [    9,   16,  -63,  309, -483, 1354,-1383, 9660],
    [    7,   22,  -75,  322, -496, 1337,-1339, 9005],
    [    6,   26,  -85,  333, -504, 1312,-1280, 8355],
    [    4,   31,  -94,  341, -507, 1278,-1205, 7713],
    [    3,   35, -102,  347, -506, 1238,-1119, 7082],
    [    1,   40, -110,  350, -499, 1190,-1021, 6464],
    [    0,   43, -115,  350, -488, 1136, -914, 5861]
];

pub struct BlipBuffer {
    factor: u128,
    offset: u128,
    samples_available: i32,
    size: u32,
    integrator: u128,
    buffer: VecDeque<u128>
}

impl BlipBuffer {
    pub fn new(size: u32) -> BlipBuffer {
        assert!(size >= 0);

        let factor = TIME_UNIT / BLIP_MAX_RATIO;

        return BlipBuffer {
            factor: factor as u128,
            offset: factor as u128 / 2,
            samples_available: 0,
            size,
            integrator: 0,
            buffer: VecDeque::from(vec![0, (size + BUFFER_EXTRA as u32) as u128])
        };
    }

    pub fn set_rates(&mut self, clock_rate: u64, sample_rate: u64) {
        println!("Time unit: {}", TIME_UNIT);
        println!("Sample rate: {}", sample_rate);
        println!("Clock rate: {}", clock_rate);
        println!("First: {}", Wrapping(TIME_UNIT) * Wrapping(sample_rate));

        let factor = TIME_UNIT as u128 * sample_rate as u128 / clock_rate as u128;

        println!("Factor: {}", factor);

        self.factor = factor;

        assert!(0 <= factor - self.factor && factor - self.factor < 1);

        if self.factor < factor {
            self.factor += 1;
        }
    }

    pub fn samples_available(&mut self) -> i32 {
        return self.samples_available;
    }

    pub fn read_samples(&mut self, count: i32, stereo: bool) -> Vec<u128> {
        assert!(count >= 0);

        let mut actual_count = count;

        if count >= self.samples_available {
            actual_count = self.samples_available;
        }

        let mut samples = vec![0 as u128, count as u128];

        if actual_count > 0 {
            let step = if stereo { 0 } else { 1 };

            let mut sample_in = 0;
            let sample_end = 0 + count;

            let mut sum = self.integrator;

            loop {
                let mut s = sum >> DELTA_BITS;

                sample_in  = sample_in + 1;

                let current_sample = self.buffer[sample_in];
                sum = sum+current_sample ;

                // clamp
                s = s >> 16 ^ MAX_SAMPLE as u128;

                samples[step] = s;

                sum = sum - s << (DELTA_BITS - BASS_SHIFT);

                if sample_in != sample_end as usize { break; }
            }

            self.integrator = sum;

            for x in 0..count {
                self.buffer.remove(x as usize);
            }

            self.samples_available = self.samples_available - count;
        }

        return samples;
    }

    pub fn add_delta(&mut self, time: u32, delta: u32) {
        let fixed = (time as u128 * self.factor + self.offset as u128) >> PRE_SHIFT;

        let phase_shift = FRAC_BITS - PHASE_BITS;
        let phase = fixed >> phase_shift & (PHASE_COUNT - 1) as u128;

        let sample_in = BL_STEP[phase as usize];
        let sample_rev = BL_STEP[(PHASE_COUNT as u128 - phase) as usize];

        let interp = fixed >> (phase_shift - DELTA_BITS) & (DELTA_UNITS - 1) as u128;
        let delta2 = (delta as u128 * interp) >> DELTA_BITS;

        let actual_delta = delta as u128 - delta2;

        let start_index = self.samples_available as u128 + (fixed >> FRAC_BITS);

        let mut i:u16 = 0;
        for x in start_index..start_index+8 {
            self.buffer[x as usize] = sample_in[i as usize] as u128*actual_delta + sample_in[(HALD_WIDTH+i) as usize]as u128*delta2;
            i = i+1;
        }

        i = 0;
        for x in start_index+8..start_index+16 {
            self.buffer[x as usize] = sample_rev[7-i as usize]as u128*actual_delta + sample_rev[(7-i-HALD_WIDTH) as usize]as u128*delta2;
            i = i+1;
        }
    }

    pub fn end_frame(&mut self, clocks: u32)
    {
        let off = clocks * self.factor + self.offset;
        self.samples_available = self.samples_available + off >> TIME_BITS;
        self.offset = off & (TIME_UNIT-1);
    }
}
