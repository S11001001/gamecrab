extern crate sdl2;
use self::sdl2::audio::{AudioCallback, AudioSpecDesired};
use cpu::*;

pub struct AudioChannel {
    pub counter: u8,
    pub enabled: bool,
    pub envelope_pos: u16,
    pub volume: u8,
}

pub struct Apu {
    pub master_clock: u32,
    pub length_clock: u32,
    pub sweep_clock: u32,
    pub envelope_clock: u32,
    pub sample_length_arr: [u8; 512],
    pub channel_1_time_freq: u32, // shadow frequency for sweeping
    pub channel_1_handle_trigger: bool,
    pub channel_1_pos: u32, // cap the lowest possible frequency to 63hz => 700 samples (GB is 64hz)
    pub channel_2_pos: u32, // cap the lowest possible frequency to 63hz => 700 samples (GB is 64hz)
    pub channel_2_handle_trigger: bool,
    pub channel_1: AudioChannel,
    pub channel_2: AudioChannel,
    pub audio_queue: sdl2::audio::AudioQueue<i16>,
    pub audio_freq: u32,
}

impl Default for AudioChannel {
    fn default() -> AudioChannel {
        AudioChannel {
            counter: 0,
            enabled: false,
            envelope_pos: 0,
            volume: 0,
        }
    }
}
impl Default for Apu {
    fn default() -> Apu {
        let audio_freq = 44100;
        Apu {
            master_clock: 0,
            length_clock: 0,
            sweep_clock: 0,
            envelope_clock: 0,
            sample_length_arr: sample_len_arr(),
            audio_queue: init_audio(44100),
            channel_1_time_freq: 0,
            channel_1_pos: 0,
            channel_1_handle_trigger: false,
            channel_2_pos: 0,
            channel_2_handle_trigger: false,
            channel_1: Default::default(),
            channel_2: Default::default(),
            audio_freq: audio_freq,
        }
    }
}


impl Apu {}
pub fn play_audio(sample_len: u8, cpu: &mut Cpu) {
    let mut result = vec![0; sample_len as usize];
    if cpu.apu.channel_1.enabled {
        mix_channel_1(&mut result, cpu);
    }
    if cpu.apu.channel_2.enabled {
        mix_channel_2(&mut result, cpu);
    }
    // mix_test(&mut result, cpu);
    cpu.apu.audio_queue.queue(&result);
}

pub fn step_length(cpu: &mut Cpu) {
    let channel_1_length_enable = read_address(0xFF14, cpu) & 0x40 != 0;
    let channel_2_length_enable = read_address(0xFF19, cpu) & 0x40 != 0;
    if cpu.apu.channel_1.enabled && channel_1_length_enable && cpu.apu.channel_1.counter != 0 {
        println!("Decrementing");
        cpu.apu.channel_1.counter -= 1;
        if cpu.apu.channel_1.counter == 0 {
            cpu.apu.channel_1.enabled = false;
        }
    }
    if cpu.apu.channel_2.enabled && channel_2_length_enable && cpu.apu.channel_2.counter != 0 {
        cpu.apu.channel_2.counter -= 1;
        if cpu.apu.channel_2.counter == 0 {
            cpu.apu.channel_2.enabled = false;
        }
    }
    // TODO do other channels
}
#[allow(non_snake_case)]
pub fn mix_channel_1(result: &mut Vec<i16>, cpu: &mut Cpu) {
    let (NR10, NR11, NR12, NR13, NR14) = read_channel_1_addresses(cpu);
    let time_freq = (NR14 as u16 & 0b111) << 8 | NR13 as u16;
    let duty = NR11 >> 6;
    let length_load = NR11 & 0x3F;
    let not_time_freq = 2048 - time_freq as u32;
    let period = (44100 * not_time_freq / 131072) as u16;
    let volume_step = (1 << 9) as i16;
    let init_volume = ((NR12 & 0xF0) >> 4) as i16;
    let sample_count = result.len();
    let volume = volume_step * init_volume;
    let high_len = get_duty(duty, period);
    for x in 0..sample_count {
        let wave_pos = (x as u16 + cpu.apu.channel_1_pos as u16) % period;
        result[x] += cond!(wave_pos <= high_len, volume, -volume);
    }
    cpu.apu.channel_1_pos = cpu.apu.channel_1_pos + sample_count as u32;
}

#[allow(non_snake_case)]
pub fn mix_test(result: &mut Vec<i16>, cpu: &mut Cpu) {
    let duty = 1;
    let not_time_freq = 440;
    let period = (44100 * not_time_freq / 131072) as u16;
    let volume_step = (1 << 9) as i16;
    let init_volume = 10 as i16;
    let sample_count = result.len();
    let volume = volume_step * init_volume;
    let high_len = get_duty(duty, period);
    for x in 0..sample_count {
        let wave_pos = (x as u16 + cpu.apu.channel_2_pos as u16) % period;
        result[x] += cond!(wave_pos <= high_len, volume, -volume);
    }
    cpu.apu.channel_2_pos = cpu.apu.channel_2_pos + sample_count as u32;
}
#[allow(non_snake_case)]
pub fn mix_channel_2(result: &mut Vec<i16>, cpu: &mut Cpu) {
    let (NR21, NR22, NR23, NR24) = read_channel_2_addresses(cpu);
    let time_freq = (NR24 as u16 & 0b111) << 8 | NR23 as u16;
    let duty = NR21 >> 6;
    let length_load = NR21 & 0x3F;
    let not_time_freq = 2048 - time_freq as u32;
    let period = (44100 * not_time_freq / 131072) as u16;
    let volume_step = (1 << 9) as i16;
    let init_volume = ((NR22 & 0xF0) >> 4) as i16;
    let sample_count = result.len();
    let volume = volume_step * init_volume;
    let high_len = get_duty(duty, period);
    for x in 0..sample_count {
        let wave_pos = (x as u16 + cpu.apu.channel_2_pos as u16) % period;
        result[x] += cond!(wave_pos <= high_len, volume, -volume);
    }
    if (cpu.apu.channel_2_pos + sample_count as u32) % period as u32 == 0 {
        cpu.apu.channel_2_pos = 0;
    } else {
        cpu.apu.channel_2_pos = cpu.apu.channel_2_pos + sample_count as u32;
    }
}

pub fn handle_triggers(cpu: &mut Cpu) {
    if cpu.apu.channel_2_handle_trigger {
        cpu.apu.channel_2.enabled = true;
        cpu.apu.channel_2.counter = 64;
        cpu.apu.channel_2.envelope_pos = 0;
        cpu.apu.channel_2_pos = 0;
        let nr22 = read_address(0xFF17, cpu);
        cpu.apu.channel_2.volume = (nr22 & 0xF0) >> 4;
        cpu.apu.channel_2_handle_trigger = false;
        cpu.apu.channel_2.enabled = nr22 & 0xF8 != 0;
    }
    if cpu.apu.channel_1_handle_trigger {
        cpu.apu.channel_1.enabled = true;
        cpu.apu.channel_1.counter = 64;
        cpu.apu.channel_1.envelope_pos = 0;
        cpu.apu.channel_1_pos = 0;
        let nr12 = read_address(0xFF12, cpu);
        cpu.apu.channel_1.volume = (nr12 & 0xF0) >> 4;
        cpu.apu.channel_1_handle_trigger = false;
        cpu.apu.channel_1.enabled = nr12 & 0xF8 != 0;
        println!("Handled {:?}", read_address(0xFF14, cpu));
    }
}
pub fn step(cpu: &mut Cpu) {
    cpu.apu.master_clock = (cpu.apu.master_clock + 1) % 512;
    let sample_len = cpu.apu.sample_length_arr[cpu.apu.master_clock as usize];
    handle_triggers(cpu);
    if cpu.apu.master_clock % 2 == 0 {
        step_length(cpu);
    }
    if cpu.apu.master_clock % 4 == 0 {
        cpu.apu.sweep_clock = (cpu.apu.sweep_clock + 1) % 512;
    }
    if cpu.apu.master_clock % 8 == 7 {
        cpu.apu.envelope_clock = (cpu.apu.envelope_clock + 1) % 512;
    }
    play_audio(sample_len, cpu);
}

pub struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

pub struct TriangleWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = match self.phase {
                0.0...0.5 => self.volume,
                _ => -self.volume,
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

impl AudioCallback for TriangleWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a triangle wave
        for x in out.iter_mut() {
            *x = -self.volume + (self.phase + self.phase) * self.volume * 2.0;
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn get_duty(duty: u8, freq: u16) -> u16 {
    match duty {
        0 => freq / 8,
        1 => freq / 4,
        2 => freq / 2,
        3 => (freq / 4) * 3,
        _ => unreachable!(),
    }
}

pub fn init_audio(freq: i32) -> sdl2::audio::AudioQueue<i16> {
    let sdl_context = sdl2::init().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(freq),
        channels: Some(1),
        // mono  -
        samples: None, // default sample size
    };

    let device = audio_subsystem.open_queue::<i16>(None, &desired_spec).unwrap();
    device.resume();
    device
}

pub fn sample_len_arr() -> [u8; 512] {
    let mut arr = [0; 512];
    for i in 0..512 {
        arr[i] = cond!(i % 128 == 0 || i % 8 == 4, 87, 86);
    }
    arr
}
