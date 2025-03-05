use anyhow::anyhow;
use std::{
    array,
    f32::consts::PI,
    ops::Not,
    sync::{Arc, Mutex},
};

use crate::circular_buffer::CircularBuffer;
pub const NOTE_NAMES: [&'static str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
pub const EMPTY_STR: &'static str = "";
pub const A4_FREQUENCY: u32 = 440;
pub enum Note {
    C = 0,
    CSharp = 1,
    D = 2,
    DSharp = 3,
    E = 4,
    F = 5,
    FSharp = 6,
    G = 7,
    GSharp = 8,
    A = 9,
    ASharp = 10,
    B = 11,
    INVALID = 12,
}

impl Note {
    fn to_str(&self) -> &'static str {
        match self {
            Note::C => NOTE_NAMES[0],
            Note::CSharp => NOTE_NAMES[1],
            Note::D => NOTE_NAMES[2],
            Note::DSharp => NOTE_NAMES[3],
            Note::E => NOTE_NAMES[4],
            Note::F => NOTE_NAMES[5],
            Note::FSharp => NOTE_NAMES[6],
            Note::G => NOTE_NAMES[7],
            Note::GSharp => NOTE_NAMES[8],
            Note::A => NOTE_NAMES[9],
            Note::ASharp => NOTE_NAMES[10],
            Note::B => NOTE_NAMES[11],
            Note::INVALID => "ZENIS",
        }
    }
    fn from_frequency(frequency: f32) -> Note {
        if frequency == 0.0 {
            return Note::INVALID;
        }

        let note_number = 12.0 * (frequency / A4_FREQUENCY as f32).log2() + 69.0;

        Self::from_number(note_number.round() as u32)
    }
    fn from_number(number: u32) -> Note {
        match number % 12 {
            0 => Note::C,
            1 => Note::CSharp,
            2 => Note::D,
            3 => Note::DSharp,
            4 => Note::E,
            5 => Note::F,
            6 => Note::FSharp,
            7 => Note::G,
            8 => Note::GSharp,
            9 => Note::A,
            10 => Note::ASharp,
            11 => Note::B,
            _ => unreachable!(),
        }
    }
}

pub struct AudioAnalyzer {
    window: Box<[f32]>,
    buffer: CircularBuffer<f32>,
    padded_buffer: Box<[f32]>,
    hps_count: usize,
    a4_freq: u32,
    sample_rate: u32,
}
pub enum WindowType {
    Hamming,
    Hann,
}

impl AudioAnalyzer {
    pub fn new(
        sample_rate: u32,
        buffer_size: usize,
        hps_count: usize,
        zero_padding_factor: usize,
        a4_freq: u32,
        window_type: WindowType,
    ) -> Self {
        let window = match window_type {
            WindowType::Hamming => Self::build_hamming_window(buffer_size),
            WindowType::Hann => Self::build_hann_window(buffer_size),
        };

        Self {
            window,
            buffer: CircularBuffer::new(buffer_size),
            padded_buffer: vec![0.0; buffer_size * zero_padding_factor].into_boxed_slice(),
            a4_freq,
            hps_count,
            sample_rate,
        }
    }

    pub fn build_hamming_window(size: usize) -> Box<[f32]> {
        (0..size)
            .map(|i| {
                let val = 0.54 - 0.46 * (2.0 * PI * i as f32 / size as f32).cos();
                val
            })
            .collect::<Box<[f32]>>()
    }

    pub fn build_hann_window(size: usize) -> Box<[f32]> {
        (0..size)
            .map(|i| {
                let val = 0.5 * (1.0 - (2.0 * PI * i as f32 / (size as f32 - 1.0))).cos();
                val
            })
            .collect::<Box<[f32]>>()
    }

    pub fn apply_window(&mut self) {
        self.buffer
            .iter_mut()
            .zip(&self.window)
            .for_each(|(sample, window_sample)| {
                *sample *= *window_sample;
            });
    }

    pub fn find_tone(&mut self) {}
}
