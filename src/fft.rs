use std::{f32::consts::PI, fmt::Error, io::Cursor, mem};

use num_complex::Complex;

use crate::{dft::TransformType, wav::WavFile};

pub struct FFT {
    data: Box<[Complex<f32>]>,
    direction: TransformType,
}

fn lower_power_of_two(n: usize) -> usize {
    if !((n & (n - 1)) != 0) {
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
        if len & (len - 1) != 0 {
            panic!()
        }
        while step < len {
            let jump = step << 1;
            let delta = match direction {
                TransformType::Forward => -PI / step as f32,
                TransformType::Inverse => PI / step as f32,
            };
            let sin = (delta * 0.5).sin();

            let multiplier = Complex::new(-2.0 * sin * sin, delta.sin());
            let mut factor = Complex::new(1.0, 0.0);

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
    fn rearrange<T>(data: &mut [T]) {
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
    pub fn freq_table(n: u32, scalar: f32) -> Box<[f32]> {
        let val = 1.0 / (n as f32 * scalar);

        let n_c = (n - 1) / 2 + 1;
        let p1 = 0..n_c as i32;
        let p2 = -((n / 2) as i32)..0;
        let result = p1.chain(p2).map(|x| x as f32 * val).collect::<Box<[f32]>>();
        result
    }
}

#[test]
fn fft() {
    let test_size: u32 = 128;
    let bin_size: f32 = 2.0;
    let test_freq: f32 = 20.0;
    let test_amp: f32 = 8.0;
    let test_scale: f32 = 2.0 / test_size as f32;
    let sample_rate = test_size as f32 / bin_size;

    let mut f0 = 0.0;
    let mut test_data = (0..test_size)
        .map(|i| {
            f0 = test_freq;
            f0 *= PI * 2.0;
            f0 *= i as f32 / sample_rate;
            f0 = test_amp * f0.cos();
            Complex::new(f0, 0.0)
        })
        .collect::<Box<[_]>>();
    FFT::rearrange(&mut test_data);
    FFT::in_place_transform(&mut test_data, TransformType::Forward, false);

    let norm_sqr = test_data
        .iter()
        .map(|value| value.norm_sqr() * test_scale)
        .collect::<Box<[_]>>();

    println!("bucket 88: {}", norm_sqr[88]);
    println!("bucket 40: {}", norm_sqr[40]);
    println!("bucket 1: {}", norm_sqr[1]);
    let len = norm_sqr.len();
    for i in 0..len {
        let freq = if i <= len / 2 {
            (i as f32) * (sample_rate / len as f32)
        } else {
            -((len - i) as f32) * (sample_rate / len as f32)
        };
        println!("Index: {}, Frequency: {}", i, freq);
    }
}

#[test]

fn rearrange() {
    let mut data = [0, 1, 2, 3, 4, 5, 6, 7];
    FFT::rearrange::<i32>(&mut data);

    assert_eq!(data, [0, 4, 2, 6, 1, 5, 3, 7])
}
