use std::{f32::consts::PI, fmt::Error, mem};

use num_complex::Complex;

use crate::dft::TransformType;

pub struct FFT {
    data: Box<[Complex<f32>]>,
    direction: TransformType,
}

fn lower_power_of_two(n: usize) -> usize {
    if (!(n & (n - 1))) != 0 {
        return n;
    } else {
        return 0x8000000000000000 >> n.leading_zeros();
    }
}

impl FFT {
    pub fn new(data: &[f32], direction: TransformType) -> Self {
        let len = if !data.len().is_power_of_two() {
            lower_power_of_two(data.len())
        } else {
            data.len()
        };

        let complex = data[0..len]
            .iter()
            .map(|value| Complex::new(*value, 0.0))
            .collect::<Box<[Complex<f32>]>>();
        Self {
            data: complex,
            direction: direction,
        }
    }

    pub fn transform(&mut self, scale: bool) -> &mut [Complex<f32>] {
        Self::rearrange(&mut self.data);
        Self::in_place_transform(&mut self.data, self.direction, scale);
        &mut self.data
    }
    pub fn fft(data: &mut [Complex<f32>], direction: TransformType, scale: bool) -> Result<(), ()> {
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
    pub fn freq_table(n: i32, scalar: f32) -> Box<[f32]> {
        let val = 1.0 / (n as f32 * scalar);

        let half_n = (n - 1) / 2 + 1;
        let p1 = 0..half_n;
        let p2 = -n / 2..0;
        let result = p1.chain(p2).map(|x| x as f32 * val).collect::<Box<[f32]>>();
        result
    }
}
