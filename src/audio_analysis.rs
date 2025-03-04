use anyhow::anyhow;
use std::{array, f32::consts::PI};
pub const NOTE_NAMES: [&'static str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
pub const EMPTY_STR: &'static str = "";
pub enum Notes {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl Notes {
    fn to_str(&self) -> &'static str {
        match self {
            Notes::C => NOTE_NAMES[0],
            Notes::CSharp => NOTE_NAMES[1],
            Notes::D => NOTE_NAMES[2],
            Notes::DSharp => NOTE_NAMES[3],
            Notes::E => NOTE_NAMES[4],
            Notes::F => NOTE_NAMES[5],
            Notes::FSharp => NOTE_NAMES[6],
            Notes::G => NOTE_NAMES[7],
            Notes::GSharp => NOTE_NAMES[8],
            Notes::A => NOTE_NAMES[9],
            Notes::ASharp => NOTE_NAMES[10],
            Notes::B => NOTE_NAMES[11],
        }
    }
}

pub struct AudioAnalyzer<const N: usize> {
    a: [f32; 2],
    b: [f32; 3],
    mem1: [f32; 4],
    mem2: [f32; 4],
    window: [f32; N],
    freq_table: [f32; N],
    note_pitch_table: [f32; N],
    note_name_table: [&'static str; N],
}

impl<const N: usize> AudioAnalyzer<N> {
    pub fn new(sample_rate: usize) -> Self {
        let (a,b) = Self::compute_second_order_low_pass_parameters(sample_rate, )
        let (mut mem1, mut mem2) = ([0.0f32; 4], [0.0f32; 4]);
        let window = Self::build_hann_window();
        let (note_name_table, freq_table, note_pitch_table) =
            Self::build_note_freq_pitch_tables(sample_rate);

        Self {
            a,
            b,
            mem1,
            mem2,
            window,
            freq_table,
            note_pitch_table,
            note_name_table,
        }
    }

    pub fn build_frequency_table(sample_rate: usize) -> [f32; N] {
        array::from_fn(|i| (sample_rate as f32 * i as f32) / N as f32)
    }
    pub fn build_note_freq_pitch_tables(
        sample_rate: usize,
    ) -> ([&'static str; N], [f32; N], [f32; N]) {
        let freq_table = Self::build_frequency_table(sample_rate);
        let mut note_name_table = [EMPTY_STR; N];
        let mut note_pitch_table = [-1.0; N];
        for i in 0..127 {
            let pitch = (440.0 / 32.0) * 2.0f32.powf((i as f32 - 9.0) / 12.0);
            if pitch > sample_rate as f32 / 2.0 {
                break;
            }
            let mut min = 100000000000000.0f32;
            let mut index: i64 = -1;

            for j in 0..N {
                if (freq_table[j as usize] - pitch).abs() < min {
                    min = (freq_table[j as usize] - pitch).abs();
                    index = j as i64;
                }
            }
            note_name_table[index as usize] = NOTE_NAMES[i as usize % 12];
            note_pitch_table[index as usize] = pitch;
        }

        (note_name_table, freq_table, note_pitch_table)
    }
    pub fn build_hamming_window() -> [f32; N] {
        array::from_fn(|i| 0.54 - 0.46 * (2.0 * PI * i as f32 / N as f32).cos())
    }

    pub fn build_hann_window() -> [f32; N] {
        array::from_fn(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (N as f32 - 1.0))).cos())
    }

    pub fn apply_window(window: &[f32; N], data: &mut [f32; N]) -> anyhow::Result<()> {
        data.iter_mut()
            .zip(window)
            .for_each(|(sample, window_sample)| {
                *sample *= window_sample;
            });
        return Ok(());
    }

    pub fn compute_second_order_low_pass_parameters(sampling_freq: f32, cutoff_freq: f32) -> ([f32; 2], [f32; 3]) {
        let mut a = [0.0f32; 2];
        let mut b = [0.0f32; 3];
        let a0: f32;
        let w0 = 2.0 * PI * cutoff_freq / sampling_freq;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * 2.0f32.sqrt();

        a0 = 1.0 + alpha;

        a[0] = (-2.0 * cos_w0) / a0;
        a[1] = (1.0 - alpha) / a0;
        b[0] = ((1.0 - cos_w0) / 2.0) / a0;
        b[1] = (1.0 - cos_w0) / a0;
        b[2] = b[0];
        (a, b)
    }

    pub fn process_second_order_filter(
        x: f32,
        mem: &mut [f32; 4],
        a: &mut [f32; 3],
        b: &mut [f32; 2],
    ) -> f32 {
        let ret = b[0] * x + b[1] * mem[0] + b[2] * mem[1] - a[0] * mem[2] - a[1] * mem[3];
        mem[1] = mem[0];
        mem[0] = x;
        mem[3] = mem[2];
        mem[2] = ret;

        return ret;
    }
}
