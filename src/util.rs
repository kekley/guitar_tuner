use std::f32::consts::PI;

use anyhow::anyhow;

pub fn build_hamming_window(size: usize) -> Box<[f32]> {
    let vec = (0..size)
        .map(|i| 0.54 - 0.46 * (2.0 * PI * i as f32 / size as f32).cos())
        .collect::<Vec<f32>>();
    vec.into_boxed_slice()
}

pub fn build_hann_window(size: usize) -> Box<[f32]> {
    let vec = (0..size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (size as f32 - 1.0))).cos())
        .collect::<Vec<f32>>();
    vec.into_boxed_slice()
}

pub fn apply_window(window: &[f32], data: &mut [f32]) -> anyhow::Result<()> {
    if window.len() != data.len() {
        return Err(anyhow!("window and data were different length"));
    } else {
        data.iter_mut()
            .zip(window)
            .for_each(|(sample, window_sample)| {
                *sample *= window_sample;
            });
        return Ok(());
    }
}

pub fn compute_second_order_low_pass_parameters(
    sample_rate: f32,
    f: f32,
    a: &mut [f32],
    b: &mut [f32],
) {
    let a0: f32;
    let w0 = 2.0 * PI * f / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * 2.0f32.sqrt();

    a0 = 1.0 + alpha;

    a[0] = (-2.0 * cos_w0) / a0;
    a[1] = (1.0 - alpha) / a0;
    b[0] = ((1.0 - cos_w0) / 2.0) / a0;
    b[1] = (1.0 - cos_w0) / a0;
    b[2] = b[0];
}

pub fn process_second_order_filter(x: f32, mem: &mut [f32], a: &mut [f32], b: &mut [f32]) -> f32 {
    let ret = b[0] * x + b[1] * mem[0] + b[2] * mem[1] - a[0] * mem[2] - a[1] * mem[3];
    mem[1] = mem[0];
    mem[0] = x;
    mem[3] = mem[2];
    mem[2] = ret;

    return ret;
}
