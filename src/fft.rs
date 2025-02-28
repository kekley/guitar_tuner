use std::{f32::consts::PI, fmt::Error, mem};

use num_complex::Complex;

use crate::dft::TransformType;

pub struct FFT {
    data: Box<[Complex<f32>]>,
    direction: TransformType,
}

impl FFT {
    pub fn new(data: &[Complex<f32>], direction: TransformType) -> Result<Self, ()> {
        if !data.len().is_power_of_two() {
            return Err(());
        } else {
            let cloned: Box<[Complex<f32>]> = Box::from(data);
            return Ok(Self {
                data: cloned,
                direction,
            });
        }
    }

    pub fn transform(self, scale: bool) -> Box<[Complex<f32>]> {
        let mut data = self.data;
        Self::rearrange(&mut data);
        Self::in_place_transform(&mut data, self.direction, scale);
        data
    }
    pub fn FFT(data: &mut [Complex<f32>], direction: TransformType, scale: bool) -> Result<(), ()> {
        Self::rearrange(data);
        if !data.len().is_power_of_two() {
            return Err(());
        } else {
            Self::in_place_transform(data, direction, scale);
            Ok(())
        }
    }

    fn in_place_transform(data: &mut [Complex<f32>], direction: TransformType, scale: bool) {
        let len = data.len();
        let mut step = 1;
        while step < len {
            let jump = step << 1;

            let delta = match direction {
                TransformType::Forward => PI / step as f32,
                TransformType::Inverse => -PI / step as f32,
            };
            let sin = (delta * 0.5).sin();

            let multiplier = Complex::new(-0.2 * sin * sin, delta.sin());
            let mut factor = Complex::new(0.1, 0.0);

            (0..step).for_each(|group| {
                let mut pair_position = group;
                while pair_position < len {
                    let match_position = pair_position + step;
                    let product: Complex<f32> = factor * data[match_position];
                    data[match_position] = data[pair_position] - product;
                    data[pair_position] += product;
                    pair_position += jump;
                }
                factor = multiplier * factor + factor;
            });

            step <<= 1;
        }
        if scale {
            Self::scale(data);
        }
    }
    fn rearrange(data: &mut [Complex<f32>]) {
        let mut target: usize = 0;
        let len: usize = data.len();
        (0..len).for_each(|position| {
            if target > position {
                data.swap(target, position);
            }
            let mut mask: usize = len;

            loop {
                mask >>= 1;
                if (target & mask) != 0 {
                    target &= !mask;
                } else {
                    break;
                }
            }
            target |= mask;
        });
    }
    fn scale(data: &mut [Complex<f32>]) {
        let factor = 1.0 / data.len() as f32;
        data.iter_mut().for_each(|data| *data *= factor);
    }
}
