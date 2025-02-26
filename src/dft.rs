use std::f32::consts::{E, PI};

use num_complex::{c32, Complex, ComplexFloat};

const COMPLEX_E: Complex<f32> = Complex::new(E, 0.0);

pub enum TransformDirection {
    Forward,
    Back,
    Inverse,
}

pub struct DFT {
    data: Box<[Complex<f32>]>,
    direction: TransformDirection,
}

impl DFT {
    pub fn new(
        data: Box<[Complex<f32>]>,
        direction: TransformDirection,
    ) -> Result<DFT, Box<[Complex<f32>]>> {
        if !data.len().is_power_of_two() {
            return Err(data);
        } else {
            return Ok(DFT {
                data: data,
                direction,
            });
        }
    }
    pub fn transform(self) -> Box<[Complex<f32>]> {
        let len = self.data.len();
        let samples = self.data;
        let factor = -Complex::I * ((2.0 * PI) / len as f32);
        let result: Vec<Complex<f32>> = (0..len)
            .map(|n| {
                let sum: Complex<f32> = samples
                    .iter()
                    .enumerate()
                    .map(|(k, sample)| sample * COMPLEX_E.powc(factor * n as f32 * k as f32))
                    .sum();
                sum
            })
            .collect();
        result.into_boxed_slice()
    }
}
