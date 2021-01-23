const PRE_SHIFT: i128 = 32;
const TIME_BITS: i128 = PRE_SHIFT + 20;

const BASS_SHIFT: i128 = 9;

const PHASE_BITS: i128 = 5;
const PHASE_COUNT: i128 = 1 << PHASE_BITS;
const DELTA_BITS: i128 = 15;
const DELTA_UNIT: i128 = 1 << DELTA_BITS;
const FRAC_BITS: i128 = TIME_BITS - PRE_SHIFT;

const TIME_UNIT: i128 = 1 << TIME_BITS;
const BLIP_MAX_RATIO: i128 = 1 << 20;

const BL_STEP: [[i128; 8]; 33] = [
    [43, -115, 350, -488, 1136, -914, 5861, 21022],
    [44, -118, 348, -473, 1076, -799, 5274, 21001],
    [45, -121, 344, -454, 1011, -677, 4706, 20936],
    [46, -122, 336, -431, 942, -549, 4156, 20829],
    [47, -123, 327, -404, 868, -418, 3629, 20679],
    [47, -122, 316, -375, 792, -285, 3124, 20488],
    [47, -120, 303, -344, 714, -151, 2644, 20256],
    [46, -117, 289, -310, 634, -17, 2188, 19985],
    [46, -114, 273, -275, 553, 117, 1758, 19675],
    [44, -108, 255, -237, 471, 247, 1356, 19327],
    [43, -103, 237, -199, 390, 373, 981, 18944],
    [42, -98, 218, -160, 310, 495, 633, 18527],
    [40, -91, 198, -121, 231, 611, 314, 18078],
    [38, -84, 178, -81, 153, 722, 22, 17599],
    [36, -76, 157, -43, 80, 824, -241, 17092],
    [34, -68, 135, -3, 8, 919, -476, 16558],
    [32, -61, 115, 34, -60, 1006, -683, 16001],
    [29, -52, 94, 70, -123, 1083, -862, 15422],
    [27, -44, 73, 106, -184, 1152, -1015, 14824],
    [25, -36, 53, 139, -239, 1211, -1142, 14210],
    [22, -27, 34, 170, -290, 1261, -1244, 13582],
    [20, -20, 16, 199, -335, 1301, -1322, 12942],
    [18, -12, -3, 226, -375, 1331, -1376, 12293],
    [15, -4, -19, 250, -410, 1351, -1408, 11638],
    [13, 3, -35, 272, -439, 1361, -1419, 10979],
    [11, 9, -49, 292, -464, 1362, -1410, 10319],
    [9, 16, -63, 309, -483, 1354, -1383, 9660],
    [7, 22, -75, 322, -496, 1337, -1339, 9005],
    [6, 26, -85, 333, -504, 1312, -1280, 8355],
    [4, 31, -94, 341, -507, 1278, -1205, 7713],
    [3, 35, -102, 347, -506, 1238, -1119, 7082],
    [1, 40, -110, 350, -499, 1190, -1021, 6464],
    [0, 43, -115, 350, -488, 1136, -914, 5861],
];

pub struct BlipBuffer {
    factor: i128,
    offset: i128,
    samples_available: i128,
    integrator: i128,
    buffer: Vec<i128>,
}

impl BlipBuffer {
    pub fn new(size: i128) -> BlipBuffer {
        let factor = TIME_UNIT / BLIP_MAX_RATIO;

        return BlipBuffer {
            factor,
            offset: 0,
            samples_available: 0,
            integrator: 0,
            buffer: Vec::from(vec![0; size as usize]),
        };
    }

    pub fn set_rates(&mut self, clock_rate: i128, sample_rate: i128) {
        self.factor = TIME_UNIT * sample_rate / clock_rate;
    }

    pub fn samples_available(&mut self) -> i128 {
        return self.samples_available;
    }

    pub fn read_samples(&mut self, count: i128, stereo: bool) -> (i128, Vec<i128>) {
        assert!(count >= 0);

        let mut actual_count = count;

        if count > self.samples_available {
            actual_count = self.samples_available;
        }

        let mut samples = vec![0; actual_count as usize];

        if actual_count > 0 {
            let step = if stereo { 2 } else { 0 };

            let mut sum = self.integrator;

            for x in 0..actual_count {
                let s = sum >> DELTA_BITS;

                let current_sample = self.buffer[x as usize];

                sum += current_sample;
                sum -= s << (DELTA_BITS - BASS_SHIFT);

                samples[(x + step) as usize] = s;
            }

            self.integrator = sum;

            self.buffer.drain(0..actual_count as usize);
            self.buffer
                .append(&mut vec![0 as i128; actual_count as usize]);

            self.samples_available -= actual_count;
        }

        return (actual_count, samples);
    }

    pub fn add_delta(&mut self, time: i128, delta: i128) {
        let fixed = (time * self.factor + self.offset) >> PRE_SHIFT;

        let phase_shift = FRAC_BITS - PHASE_BITS;
        let phase = fixed >> phase_shift & (PHASE_COUNT - 1);

        let sample_in = BL_STEP[phase as usize];
        let sample_in_half = BL_STEP[(phase + 1) as usize];
        let sample_rev = BL_STEP[(PHASE_COUNT - phase) as usize];
        let sample_rev_half = BL_STEP[((PHASE_COUNT - phase) - 1) as usize];

        let interp = fixed >> (phase_shift - DELTA_BITS) & (DELTA_UNIT - 1);
        let delta2 = (delta * interp) >> DELTA_BITS;

        let actual_delta = delta - delta2;

        let start_index = self.samples_available + (fixed >> FRAC_BITS);

        let mut i: i128 = 0;
        for x in start_index..start_index + 8 {
            self.buffer[x as usize] +=
                sample_in[i as usize] * actual_delta + sample_in_half[i as usize] * delta2;
            i = i + 1;
        }

        i = 7;
        for x in start_index + 8..start_index + 16 {
            self.buffer[x as usize] +=
                sample_rev[i as usize] * actual_delta + sample_rev_half[i as usize] * delta2;
            i = i - 1;
        }
    }

    pub fn end_frame(&mut self, clocks: i128) {
        let off = clocks * self.factor + self.offset;
        self.samples_available = self.samples_available + off >> TIME_BITS;
        self.offset = off & (TIME_UNIT - 1);
    }
}
