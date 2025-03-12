use std::{
    f32::consts::{E, PI},
    io::Cursor,
};

use num_complex::{c32, Complex, ComplexFloat};

use crate::wav::WavFile;

const COMPLEX_E: Complex<f32> = Complex::new(E, 0.0);

#[derive(Debug, Clone, Copy)]
pub enum TransformType {
    Forward,
    Inverse,
}

pub struct DFT {
    data: Box<[Complex<f32>]>,
    direction: TransformType,
}

impl DFT {
    pub fn new(
        data: Box<[Complex<f32>]>,
        direction: TransformType,
    ) -> Result<DFT, Box<[Complex<f32>]>> {
        if !data.len().is_power_of_two() {
            return Err(data);
        } else {
            return Ok(DFT { data, direction });
        }
    }

    pub fn transform(self) -> Box<[Complex<f32>]> {
        match self.direction {
            TransformType::Forward => Self::forward_transform(self.data),
            TransformType::Inverse => Self::inverse_transform(self.data),
        }
    }

    fn forward_transform(data: Box<[Complex<f32>]>) -> Box<[Complex<f32>]> {
        let len: usize = data.len();
        let samples: Box<[Complex<f32>]> = data;
        let factor: Complex<f32> = -Complex::I * ((2.0 * PI) / len as f32);
        let result: Vec<Complex<f32>> = (0..len)
            .map(|n: usize| {
                let sum: Complex<f32> = samples
                    .iter()
                    .enumerate()
                    .map(|(k, sample)| sample * COMPLEX_E.powc(factor * n as f32 * k as f32))
                    .sum();
                sum
            })
            .collect::<Vec<_>>();
        result.into_boxed_slice()
    }
    fn inverse_transform(data: Box<[Complex<f32>]>) -> Box<[Complex<f32>]> {
        let len: usize = data.len();
        let samples: Box<[Complex<f32>]> = data;
        let factor: Complex<f32> = Complex::I * ((2.0 * PI) / len as f32);
        let result: Vec<Complex<f32>> = (0..len)
            .map(|n: usize| {
                let sum: Complex<f32> = samples
                    .iter()
                    .enumerate()
                    .map(|(k, sample)| sample * COMPLEX_E.powc(factor * n as f32 * k as f32))
                    .sum();
                (1.0 / len as f32) * sum
            })
            .collect::<Vec<_>>();
        result.into_boxed_slice()
    }
}
