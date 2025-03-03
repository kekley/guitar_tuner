pub const NOTE_NAMES: [&'static str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

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
    b: [f32; 2],
    mem1: [f32; 4],
    mem2: [f32; 4],
    window: [f32; N],
    freq_table: [f32; N],
    note_pitch_table: [f32; N],
    note_name_table: [&'static str; N],
}

impl<const N: usize> AudioAnalyzer<N> {
    pub fn new(sample_rate: usize) -> Self {
        todo!()
    }
}
