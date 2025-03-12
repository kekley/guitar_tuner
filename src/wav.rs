use std::{
    ffi::CStr,
    num::{NonZero, NonZeroU32},
    str::Bytes,
};

const RIFF_HEADER: [u8; 4] = [0x52, 0x49, 0x46, 0x46]; //RIFF
const RIFX_HEADER: [u8; 4] = [0x52, 0x49, 0x46, 0x58]; //RIFX
const WAVE_HEADER: [u8; 4] = [0x57, 0x41, 0x56, 0x45]; //WAVE
const FMT_HEADER: [u8; 4] = [0x66, 0x6D, 0x74, 0x20]; //FMT
const FACT_HEADER: [u8; 4] = [0x66, 0x61, 0x63, 0x74]; //fact
const PEAK_HEADER: [u8; 4] = [0x50, 0x45, 0x41, 0x4B]; //PEAK

#[repr(u16)]
pub enum WavFormat {
    PCM = 0x0001,
    Float = 0x0003,
    ALaw = 0x0006,
    MuLaw = 0x0007,
    Extensible = 0xFFFE,
}

#[repr(C)]
struct FmtChunk {
    fmt_str: [u8; 4],
    chunk_size: u32,
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    extended: Option<u16>,
}
#[repr(C)]
struct FactChunk {
    fact_str: [u8; 4],
    chunk_size: u32,
    sample_length: u32,
}
#[repr(C)]
struct PeakChunk {
    chunk_size: u32,
    version: u32,
    time_stamp: u32,
    peak: PositionPeak,
}

#[repr(C)]
struct DataChunk {}
#[repr(C)]
struct PositionPeak {
    value: f32,
    position: u32,
}

pub struct WavHeader {
    riff_header: [u8; 4],
    file_size: u32,
    wave_header: [u8; 4],
    fact_chunk: Option<FactChunk>,
    peak_chunk: Option<PeakChunk>,
    data_header: [u8; 4],
}
