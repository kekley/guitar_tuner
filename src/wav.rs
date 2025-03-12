use core::fmt;
use std::{
    ffi::CStr,
    io::{Cursor, Read, Seek, SeekFrom},
    num::{NonZero, NonZeroU32},
    str::{self, Bytes},
};

use anyhow::{anyhow, Ok};
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use imgui_glow_renderer::glow::COPY_READ_BUFFER;
use sdl2::libc::SOCKET;

const RIFF_HEADER: [u8; 4] = [0x52, 0x49, 0x46, 0x46]; //RIFF
const RIFX_HEADER: [u8; 4] = [0x52, 0x49, 0x46, 0x58]; //RIFX
const WAVE_HEADER: [u8; 4] = [0x57, 0x41, 0x56, 0x45]; //WAVE
const FMT_HEADER: [u8; 4] = [0x66, 0x6D, 0x74, 0x20]; //FMT
const FACT_HEADER: [u8; 4] = [0x66, 0x61, 0x63, 0x74]; //fact
const PEAK_HEADER: [u8; 4] = [0x50, 0x45, 0x41, 0x4B]; //PEAK
const DATA_HEADER: [u8; 4] = [0x64, 0x61, 0x74, 0x61]; //data

#[derive(Debug)]
#[repr(u16)]
pub enum WavFormat {
    PCM = 0x0001,
    Float = 0x0003,
    ALaw = 0x0006,
    MuLaw = 0x0007,
    Extensible = 0xFFFE,
    INVALID = 0xFFFF,
}
impl WavFormat {
    fn from_u16(value: u16) -> Result<Self, anyhow::Error> {
        match value {
            1 => Ok(Self::PCM),
            3 => Ok(Self::Float),
            6 => Ok(Self::ALaw),
            7 => Ok(Self::MuLaw),
            65534 => Ok(Self::Extensible),
            _ => Err(anyhow!("Unknown value for WavFormat")),
        }
    }
}

enum RiffChunk {
    Fmt(FmtChunk),
    Fact(FactChunk),
    Peak(PeakChunk),
    Data(DataChunk),
}

impl RiffChunk {
    fn is_fmt(&self) -> bool {
        match self {
            RiffChunk::Fmt(_) => true,
            _ => false,
        }
    }
    fn is_fact(&self) -> bool {
        match self {
            RiffChunk::Fact(_) => true,
            _ => false,
        }
    }
    fn is_peak(&self) -> bool {
        match self {
            RiffChunk::Peak(_) => true,
            _ => false,
        }
    }
    fn is_data(&self) -> bool {
        match self {
            RiffChunk::Data(_) => true,
            _ => false,
        }
    }
    fn take_fmt(self) -> Option<FmtChunk> {
        match self {
            RiffChunk::Fmt(chunk) => Some(chunk),
            _ => None,
        }
    }
    fn take_fact(self) -> Option<FactChunk> {
        match self {
            RiffChunk::Fact(chunk) => Some(chunk),
            _ => None,
        }
    }
    fn take_peak(self) -> Option<PeakChunk> {
        match self {
            RiffChunk::Peak(chunk) => Some(chunk),
            _ => None,
        }
    }
    fn take_data(self) -> Option<DataChunk> {
        match self {
            RiffChunk::Data(chunk) => Some(chunk),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct FmtChunk {
    fmt_str: [u8; 4],
    chunk_size: u32,
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    extra_data_size: Option<u16>,
    extra_data: Option<Box<[u8]>>,
}
impl FmtChunk {
    fn read<T: Read + Seek>(data: &mut T, header: [u8; 4]) -> Result<Self, anyhow::Error> {
        let chunk_size = data.read_u32::<LittleEndian>()?;
        let format = WavFormat::from_u16(data.read_u16::<LittleEndian>()?)?;
        let channels = data.read_u16::<LittleEndian>()?;
        let sample_rate = data.read_u32::<LittleEndian>()?;
        let byte_rate = data.read_u32::<LittleEndian>()?;
        let block_align = data.read_u16::<LittleEndian>()?;
        let bits_per_sample = data.read_u16::<LittleEndian>()?;
        let (extra_data_size, extra_data) = if chunk_size != 16 {
            //not standard, we need to read extra data
            let extra_data_size = data.read_u16::<LittleEndian>()?;
            let bytes_remaining = bytes_remaining(data)?;
            if bytes_remaining < extra_data_size as u64 {
                return Err(anyhow!("Unexpected EOF reading extra fmt data"));
            }
            let extra_data = (0..extra_data_size)
                .map(|_| data.read_u8().unwrap())
                .collect::<Box<_>>();
            (Some(extra_data_size), Some(extra_data))
        } else {
            (None, None)
        };
        Ok(Self {
            fmt_str: header,
            chunk_size,
            format,
            channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
            extra_data_size,
            extra_data,
        })
    }
}
#[derive(Debug)]
struct FactChunk {
    fact_str: [u8; 4],
    chunk_size: u32,
    data: u32, // typically number of samples
}

impl FactChunk {
    fn read<T: Read + Seek>(data: &mut T, header: [u8; 4]) -> Result<Self, anyhow::Error> {
        let chunk_size = data.read_u32::<LittleEndian>()?;
        let data = data.read_u32::<LittleEndian>()?;
        Ok(Self {
            fact_str: header,
            chunk_size,
            data,
        })
    }
}
#[derive(Debug)]
struct PeakChunk {
    peak_str: [u8; 4],
    chunk_size: u32,
    version: u32,
    time_stamp: u32,
    peak: PositionPeak,
}
impl PeakChunk {
    fn read<T: Read + Seek>(data: &mut T, header: [u8; 4]) -> Result<Self, anyhow::Error> {
        let chunk_size = data.read_u32::<LittleEndian>()?;
        let version = data.read_u32::<LittleEndian>()?;
        let time_stamp = data.read_u32::<LittleEndian>()?;
        let peak = PositionPeak::read(data)?;
        Ok(Self {
            peak_str: header,
            chunk_size,
            version,
            time_stamp,
            peak: peak,
        })
    }
}
#[derive(Debug)]
struct PositionPeak {
    value: u32,
    position: u32,
}
impl PositionPeak {
    fn read<T: Read + Seek>(data: &mut T) -> Result<Self, anyhow::Error> {
        let value = data.read_u32::<LittleEndian>()?;
        let position = data.read_u32::<LittleEndian>()?;

        Ok(Self { value, position })
    }
}

struct DataChunk {
    data_str: [u8; 4],
    chunk_size: u32,
    data: Box<[u8]>,
}
impl fmt::Debug for DataChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataChunk")
            .field("data_str", &self.data_str)
            .field("chunk_size", &self.chunk_size)
            .field("data", &self.data.as_ptr())
            .finish()
    }
}

impl DataChunk {
    fn read<T: Read + Seek>(data: &mut T, header: [u8; 4]) -> Result<Self, anyhow::Error> {
        let chunk_size = data.read_u32::<LittleEndian>()?;
        let mut vec = vec![0; chunk_size as usize];
        data.read_exact(&mut vec[..])?;
        Ok(Self {
            data_str: header,
            chunk_size,
            data: vec.into_boxed_slice(),
        })
    }
}

#[derive(Debug)]
pub struct WavFile {
    riff_header: [u8; 4],
    file_size: u32,
    wave_header: [u8; 4],
    fmt_chunk: FmtChunk,
    fact_chunk: Option<FactChunk>,
    peak_chunk: Option<PeakChunk>,
    data_chunk: DataChunk,
}

fn parse_chunk<'a, T: Read + Seek>(data: &'a mut T) -> Result<RiffChunk, anyhow::Error> {
    let mut header = [0u8; 4];
    data.read(&mut header)?;

    match header {
        FMT_HEADER => Ok(RiffChunk::Fmt(FmtChunk::read(data, header)?)),
        PEAK_HEADER => Ok(RiffChunk::Peak(PeakChunk::read(data, header)?)),
        FACT_HEADER => Ok(RiffChunk::Fact(FactChunk::read(data, header)?)),
        DATA_HEADER => Ok(RiffChunk::Data(DataChunk::read(data, header)?)),
        _ => {
            return Err(anyhow!("Unsupported header found"));
        }
    }
}

fn bytes_remaining<T: Read + Seek>(data: &mut T) -> Result<u64, anyhow::Error> {
    let old_pos = data.stream_position()?;
    let end = data.seek(SeekFrom::End(0))?;
    let bytes_remaining = end - old_pos;
    // Avoid seeking a third time when we were already at the end of the
    // stream. The branch is usually way cheaper than a seek operation.
    if old_pos != end {
        data.seek(SeekFrom::Start(old_pos))?;
    }
    Ok(bytes_remaining)
}

impl WavFile {
    pub fn from_bytes<T: Read + Seek>(data: &mut T) -> Result<Self, anyhow::Error> {
        let mut riff_header = [0u8; 4];

        data.read_exact(&mut riff_header)?;
        let data_len = data.read_u32::<LittleEndian>()?;

        let bytes_left = bytes_remaining(data)?;

        if data_len as u64 != bytes_left {
            return Err(anyhow!("file size different from size specified in header"));
        };
        let mut wave_header = [0u8; 4];

        data.read_exact(&mut wave_header)?;

        if riff_header != RIFF_HEADER {
            return Err(anyhow!("bad RIFF header"));
        }

        if wave_header != WAVE_HEADER {
            return Err(anyhow!("bad WAVE header"));
        }

        let mut chunks: Vec<RiffChunk> = vec![];
        while bytes_remaining(data)? > 0 {
            chunks.push(parse_chunk(data)?);
        }

        let mut fmt_chunk: Option<FmtChunk> = None;
        let mut data_chunk: Option<DataChunk> = None;
        let mut peak_chunk: Option<PeakChunk> = None;
        let mut fact_chunk: Option<FactChunk> = None;

        for chunk in chunks.into_iter() {
            match chunk {
                RiffChunk::Fmt(fmt) => fmt_chunk = Some(fmt),
                RiffChunk::Data(data) => data_chunk = Some(data),
                RiffChunk::Peak(peak) => peak_chunk = Some(peak),
                RiffChunk::Fact(fact) => fact_chunk = Some(fact),
            }
        }

        Ok(Self {
            riff_header,
            file_size: data_len,
            wave_header,
            fmt_chunk: fmt_chunk.ok_or(anyhow!("No fmt chunk"))?,
            fact_chunk,
            peak_chunk,
            data_chunk: data_chunk.ok_or(anyhow!("No data chunk"))?,
        })
    }

    pub fn get_samples(&self) -> &[f32] {
        let data = self.data_chunk.data.as_ref();
        let a = unsafe { data.align_to::<f32>() };
        a.1
    }
}
#[test]
fn headers() {
    assert_eq!(str::from_utf8(&RIFF_HEADER).unwrap(), "RIFF");
    assert_eq!(str::from_utf8(&RIFX_HEADER).unwrap(), "RIFX");
    assert_eq!(str::from_utf8(&PEAK_HEADER).unwrap(), "PEAK");
    assert_eq!(str::from_utf8(&FACT_HEADER).unwrap(), "fact");
    assert_eq!(str::from_utf8(&FMT_HEADER).unwrap(), "fmt ");
    assert_eq!(str::from_utf8(&WAVE_HEADER).unwrap(), "WAVE");
}

#[test]
fn file_read() {
    let file = include_bytes!(".././A.wav");

    let mut cursor = Cursor::new(file);

    let wav = WavFile::from_bytes(&mut cursor).unwrap();
}
