use anyhow::anyhow;
use num_complex::{Complex, ComplexFloat};
use std::{
    array,
    f32::consts::PI,
    io::{copy, Cursor},
    ops::Not,
    sync::{Arc, Mutex},
};

use crate::{circular_buffer::CircularBuffer, fft::FFT, wav::WavFile};
pub const NOTE_NAMES: [&'static str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
pub const EMPTY_STR: &'static str = "";
pub const A4_FREQUENCY: u32 = 440;

#[derive(Debug, Clone, Copy)]
pub enum SampleRate {
    KHz44_1 = 44100,
    KHz48 = 48000,
    KHz88_2 = 88200,
    KHz96 = 96000,
}

impl SampleRate {
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}
#[derive(Debug, PartialEq, Eq)]
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
    pub fn number_to_freq(number: u32, a4_freq: u32) -> f32 {
        a4_freq as f32 * 2.0.powf((number - 69) as f32 / 12.0)
    }
    pub fn to_str(&self) -> &'static str {
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

    pub fn add_samples(&mut self, samples: &[f32]) {
        samples.iter().for_each(|sample| {
            self.buffer.push_back(*sample);
        });
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
                let val = 0.5 * (1.0 - (2.0 * PI * i as f32 / (size as f32 - 1.0)).cos());
                val
            })
            .collect::<Box<[f32]>>()
    }

    fn make_complex_padded(&mut self) {
        let len = self.buffer.len();
        self.buffer
            .iter()
            .zip(self.window.iter())
            .zip(self.padded_buffer.iter_mut())
            .for_each(|((sample, window_value), dest)| *dest = sample * window_value);
        self.padded_buffer
            .iter_mut()
            .skip(len)
            .for_each(|should_be_zero| {
                *should_be_zero = 0.0;
            });
    }

    pub fn find_tone(&mut self) -> Note {
        self.make_complex_padded();
        let mut fft = FFT::new(&self.padded_buffer, crate::dft::TransformType::Forward);
        let mut result = fft
            .transform(false)
            .iter_mut()
            .map(|f| f.abs())
            .collect::<Box<[f32]>>();
        let half_len = result.len() / 2;
        let half_data = &mut result[0..half_len];

        let freq_table = FFT::freq_table(
            (half_data.len() * 2).try_into().unwrap(),
            1.0 / self.sample_rate as f32,
        );

        for (i, freq) in freq_table.iter().enumerate() {
            if *freq > 60.0 {
                half_data[..i - 1].iter_mut().for_each(|f| *f = 0.0);
                break;
            }
        }

        let loudest_tone_index = half_data
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .unwrap();

        let loudest_freq = (freq_table[loudest_tone_index] * 100.0).round() / 100.0;
        let note = Note::from_frequency(loudest_freq);
        note
    }
}

#[test]
fn test_analysis() {
    let bytes = include_bytes!(".././A.wav");
    let mut cursor = Cursor::new(bytes);
    let wav = WavFile::from_bytes(&mut cursor).unwrap();

    let mut analyzer = AudioAnalyzer::new(
        SampleRate::KHz48.to_u32(),
        1024 * 50,
        3,
        3,
        440,
        WindowType::Hann,
    );

    analyzer.add_samples(wav.get_samples());
    let a = analyzer.find_tone();
    assert_eq!(a, Note::A);
    let bytes = include_bytes!(".././A_RECORDING.wav");
    let mut cursor = Cursor::new(bytes);
    let wav = WavFile::from_bytes(&mut cursor).unwrap();

    let mut analyzer = AudioAnalyzer::new(
        SampleRate::KHz48.to_u32(),
        1024 * 50,
        3,
        3,
        440,
        WindowType::Hann,
    );

    analyzer.add_samples(wav.get_samples());
    let a = analyzer.find_tone();
    assert_eq!(a, Note::A);
    let bytes = include_bytes!(".././B.wav");
    let mut cursor = Cursor::new(bytes);
    let wav = WavFile::from_bytes(&mut cursor).unwrap();

    let mut analyzer = AudioAnalyzer::new(
        SampleRate::KHz48.to_u32(),
        1024 * 50,
        3,
        3,
        440,
        WindowType::Hann,
    );

    analyzer.add_samples(wav.get_samples());
    let b = analyzer.find_tone();
    assert_eq!(b, Note::B);
}

#[test]
fn test_note_to_str() {
    let freq = Note::number_to_freq(69, 440);
    let note = Note::from_frequency(freq).to_str();
    assert_eq!(freq, 440.0);
    assert_eq!(note, "A");
    println!("freq: {}", freq);
    println!("note: {}", note);
}
