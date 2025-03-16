use std::{
    fmt::{format, Debug},
    io::Cursor,
    ops::{Deref, Range},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, Error, Ok};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, FromSample, Host, Sample, SampleFormat, Stream, StreamConfig, ALL_HOSTS,
};
use imgui::{Context, Ui};
use imgui_glow_renderer::{
    glow::{self, HasContext, COLOR_BUFFER_BIT},
    AutoRenderer,
};
use imgui_sdl2_support::SdlPlatform;
use num_complex::ComplexFloat;
use plotters::{
    chart::ChartBuilder,
    prelude::{BitMapBackend, IntoDrawingArea},
    series::LineSeries,
    style::{
        full_palette::{BLACK, RED_700},
        Color, WHITE,
    },
};
use sdl2::{
    event::{self, Event, WindowEvent},
    video::{gl_attr::GLAttr, GLContext, Window},
    EventPump, Sdl, VideoSubsystem,
};

use crate::{
    audio_analysis::{find_max_float, AudioAnalyzer, Note},
    circular_buffer::CircularBuffer,
    fft::FFT,
    wav::WavFile,
};

pub const DEFAULT_WINDOW_TITLE: &str = "dodge left dodge right";
pub const DEFAULT_WIDTH: usize = 800;
pub const DEFAULT_HEIGHT: usize = 600;
pub const BUFFER_SIZE: usize = 8192;
unsafe fn get_glow_context(window: &Window) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function(|s| window.subsystem().gl_get_proc_address(s) as _)
    }
}

fn initialize_sdl() -> anyhow::Result<(Sdl, VideoSubsystem)> {
    let sdl = sdl2::init().map_err(|error| anyhow!(error))?;
    let video_subsystem = sdl.video().map_err(|error| anyhow!(error))?;

    let gl_attr = video_subsystem.gl_attr();

    //opengl 3.3
    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    Ok((sdl, video_subsystem))
}

fn create_window(
    sdl_video: &VideoSubsystem,
    window_title: &str,
    window_width: usize,
    window_height: usize,
) -> anyhow::Result<Window> {
    let window = sdl_video
        .window(window_title, window_width as u32, window_height as u32)
        .allow_highdpi()
        .opengl()
        .position_centered()
        .resizable()
        .build()?;

    Ok(window)
}

fn create_opengl_context(window: &Window) -> anyhow::Result<GLContext> {
    let context = window.gl_create_context().map_err(|error| anyhow!(error))?;
    window
        .gl_make_current(&context)
        .map_err(|error| anyhow!(error))?;
    Ok(context)
}

fn init_imgui(
    glow_context: glow::Context,
) -> anyhow::Result<(imgui::Context, SdlPlatform, AutoRenderer)> {
    let mut imgui = imgui::Context::create();

    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);

    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    let platform = SdlPlatform::new(&mut imgui);

    let renderer = AutoRenderer::new(glow_context, &mut imgui)?;

    Ok((imgui, platform, renderer))
}

#[derive(Debug, Default)]
pub struct AppBuilder {
    window_title: Option<String>,
    window_width: Option<usize>,
    window_height: Option<usize>,
    vsync_enabled: Option<bool>,
}
impl AppBuilder {
    pub fn window_title(self, title: &str) -> Self {
        Self {
            window_title: Some(title.to_owned()),
            ..self
        }
    }
    pub fn window_width(self, width: usize) -> Self {
        Self {
            window_width: Some(width),
            ..self
        }
    }
    pub fn window_height(self, height: usize) -> Self {
        Self {
            window_height: Some(height),
            ..self
        }
    }
    pub fn vsync_enabled(self, enabled: bool) -> Self {
        Self {
            vsync_enabled: Some(enabled),
            ..self
        }
    }

    pub fn build(self) -> anyhow::Result<App> {
        let (sdl, video_subsystem) = initialize_sdl()?;
        let window = create_window(
            &video_subsystem,
            &self
                .window_title
                .unwrap_or(DEFAULT_WINDOW_TITLE.to_string()),
            self.window_width.unwrap_or(DEFAULT_WIDTH),
            self.window_height.unwrap_or(DEFAULT_HEIGHT),
        )?;

        let gl_context = create_opengl_context(&window)?;

        if self.vsync_enabled.unwrap_or(true) {
            window
                .subsystem()
                .gl_set_swap_interval(1)
                .map_err(|error| anyhow!(error))?;
        }

        let glow_context = unsafe { get_glow_context(&window) };

        let (imgui_context, imgui_platform, imgui_renderer) = init_imgui(glow_context)?;

        let event_pump = sdl.event_pump().map_err(|error| anyhow!(error))?;
        let audio_host = cpal::default_host();
        let app = App {
            sdl,
            video_subsystem,
            window,
            gl_context,
            imgui_context,
            imgui_platform,
            imgui_renderer,
            event_pump,
            audio_host,
            sample_buffer: Arc::new(Mutex::new(CircularBuffer::new(2048))),
            audio_analyzer: Arc::new(Mutex::new(AudioAnalyzer::new(
                48000,
                1024 * 50,
                3,
                3,
                440,
                crate::audio_analysis::WindowType::Hann,
            ))),
        };

        Ok(app)
    }
}

/*         let mut window_size_x = window_size_x as f32;
       let mut window_size_y = window_size_y as f32;
       let mut device_number: i32 = 0;
       let mut current_stream: Option<Stream> = None;
       let mut device_list: Vec<Device> = vec![];
       let mut device_names: Vec<Box<str>> = vec![];
       let mut need_device_refresh = true;
       let mut sample_buffer = Arc::new(Mutex::new(CircularBuffer::<f32>::new(BUFFER_SIZE)));
*/
struct AppContext {
    window_size_x: u32,
    window_size_y: u32,
    device_number: i32,
    current_stream: Option<Stream>,
    device_list: Vec<Device>,
    device_names: Vec<String>,
    need_device_refresh: bool,
}

impl AppContext {
    fn new(app: &App) -> Self {
        let size = app.window.size();
        Self {
            window_size_x: size.0,
            window_size_y: size.1,
            device_number: 0,
            current_stream: None,
            device_list: vec![],
            device_names: vec![],
            need_device_refresh: true,
        }
    }
}
pub struct App {
    sdl: Sdl,
    video_subsystem: VideoSubsystem,
    window: Window,
    gl_context: GLContext,
    imgui_context: imgui::Context,
    imgui_platform: SdlPlatform,
    imgui_renderer: AutoRenderer,
    event_pump: EventPump,
    audio_host: Host,
    sample_buffer: Arc<Mutex<CircularBuffer<f32>>>,
    audio_analyzer: Arc<Mutex<AudioAnalyzer>>,
}

impl App {
    pub fn new() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn run(self) -> anyhow::Result<()> {
        let mut context = AppContext::new(&self);
        let App {
            mut sdl,
            mut video_subsystem,
            mut window,
            mut gl_context,
            mut imgui_context,
            mut imgui_platform,
            mut imgui_renderer,
            mut event_pump,
            audio_host,
            sample_buffer,
            audio_analyzer,
        } = self;
        let mut tone_hit_counter = 0;
        let mut nearest_note_num_buffer: f32 = 0.0;
        let mut note_number_counter = 0;
        'main: loop {
            for event in event_pump.poll_iter() {
                //event passed to imgui
                imgui_platform.handle_event(&mut imgui_context, &event);

                if let Event::Quit { .. } = event {
                    break 'main;
                }
                if let Event::AudioDeviceAdded { .. } = event {
                    context.need_device_refresh = true;
                }
                if let Event::AudioDeviceRemoved { .. } = event {
                    context.need_device_refresh = true;
                }

                if let Event::Window {
                    win_event,
                    timestamp: _,
                    window_id: _,
                } = event
                {
                    match win_event {
                        WindowEvent::SizeChanged(x, y) => {
                            context.window_size_x = x as u32;
                            context.window_size_y = y as u32;
                        }
                        _ => {}
                    }
                }
            }
            if context.need_device_refresh {
                Self::refresh_device_list(
                    &audio_host,
                    &mut context.device_list,
                    &mut context.device_names,
                );
            }
            if context.current_stream.is_none() {
                let swap_succeeded = Self::swap_device(
                    &mut context.current_stream,
                    &mut context.device_list,
                    context.device_number,
                    &audio_analyzer,
                    &sample_buffer,
                );
                if swap_succeeded.is_err() {
                    println!("fuck")
                }
            }

            imgui_platform.prepare_frame(&mut imgui_context, &window, &event_pump);
            let ui = imgui_context.new_frame();
            let mut analyzer_guard = audio_analyzer.lock().unwrap();
            let mut buffer_guard = sample_buffer.lock().unwrap();
            let sample_data = buffer_guard
                .make_contiguous()
                .iter()
                .cloned()
                .collect::<Box<_>>();

            let strongest_freq = analyzer_guard.strongest_freq();
            drop(analyzer_guard);
            drop(buffer_guard);
            let number = Note::freq_to_number(strongest_freq, 440);

            let nearest_note_number = number.round();
            let nearest_freq = Note::number_to_freq(nearest_note_number, 440);

            let freq_difference = nearest_freq - strongest_freq;

            let semitone_step = nearest_freq - Note::number_to_freq((number - 1.0).round(), 440);

            let mut diff_cents = if semitone_step == 0.0 {
                0.0
            } else {
                (freq_difference / semitone_step) * 100.0
            };
            if nearest_note_number != nearest_note_num_buffer {
                note_number_counter += 1;
                if note_number_counter >= 7 {
                    nearest_note_num_buffer = nearest_note_number;
                    note_number_counter = 0;
                }
            }

            diff_cents = ((diff_cents * 10.0).round()) / 10.0;

            ///////////////////////////////////////////////
            //ui code  goes here

            Self::draw_sample_graph(
                &sample_data,
                &ui,
                context.window_size_x as f32,
                context.window_size_y as f32,
            );
            Self::draw_note_data(
                ui,
                context.window_size_x as f32,
                context.window_size_y as f32,
                nearest_note_num_buffer,
                diff_cents,
            );
            if Self::draw_device_list(&mut context, &ui) {
                context.current_stream.unwrap().pause()?;
                context.current_stream = None;
            }

            //////////////////////////////////////////////
            let draw_data = imgui_context.render();

            unsafe {
                imgui_renderer.gl_context().clear(COLOR_BUFFER_BIT);
            }

            imgui_renderer
                .render(draw_data)
                .map_err(|error| anyhow!(error))?;

            window.gl_swap_window();
        }
        Ok(())
    }
    fn write_callback(
        input: &[f32],
        buffer: &Arc<Mutex<CircularBuffer<f32>>>,
        analyzer: &Arc<Mutex<AudioAnalyzer>>,
    ) {
        let (mut buffer, mut analyzer) = (buffer.lock().unwrap(), analyzer.lock().unwrap());
        (0..input.len()).for_each(|i| {
            let _ = buffer.push_back(input[i]);
        });
        if buffer.len() >= 1024 {
            analyzer.add_samples(&buffer.drain(0..1024).collect::<Box<_>>());
        }
    }

    pub fn save_chart(data: &[f32]) {
        let (ind, max) = find_max_float(data);
        let root = BitMapBackend::new("../plot.png", (1920, 1080)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .build_cartesian_2d(0f32..data.len() as f32, 0.0f32..*max)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        let iter = (0..data.len())
            .zip(data.iter())
            .map(|(x, y)| (x as f32, *y));
        chart
            .draw_series(LineSeries::new(iter, &plotters::style::RED))
            .unwrap();

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()
            .unwrap();

        root.present().unwrap();
    }

    fn draw_sample_graph(samples: &[f32], ui: &Ui, window_size_x: f32, window_size_y: f32) {
        let _ = ui
            .window("Audio Samples")
            .resizable(true)
            .movable(true)
            .position([window_size_x / 2.0, 0.0], imgui::Condition::FirstUseEver)
            .size(
                [window_size_x / 2.0, window_size_y / 2.0],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                let plot = ui
                    .plot_lines("Sample Data:", &samples)
                    .scale_max(1.0)
                    .scale_min(-1.0)
                    .graph_size([
                        (window_size_x / 2.0) - window_size_x * 0.1,
                        (window_size_y / 2.0) - window_size_x * 0.1,
                    ]);
                plot.build();
            });
    }

    fn draw_note_data(
        ui: &Ui,
        window_size_x: f32,
        window_size_y: f32,
        nearest_note: f32,
        diff_cents: f32,
    ) {
        let diff_cents = if diff_cents >= 0.0 {
            format!("-{}", diff_cents)
        } else {
            format!("{}", diff_cents)
        };
        let _ = ui
            .window("Note Data:")
            .resizable(true)
            .movable(true)
            .position(
                [window_size_x / 2.0, window_size_y / 2.0],
                imgui::Condition::FirstUseEver,
            )
            .size(
                [window_size_x / 2.0, window_size_y / 2.0],
                imgui::Condition::FirstUseEver,
            )
            .build(|| {
                ui.text(format!(
                    "Nearest Note: {}\nNearest Note Number: {}\nDifference in cents: {}",
                    Note::from_frequency(Note::number_to_freq(nearest_note, 440)).to_str(),
                    nearest_note,
                    diff_cents
                ));
            });
    }

    fn refresh_device_list(host: &Host, devices: &mut Vec<Device>, device_names: &mut Vec<String>) {
        devices.clear();
        device_names.clear();
        for (device_num, device) in host
            .input_devices()
            .expect("could not get device list")
            .enumerate()
        {
            let device_name = device
                .name()
                .unwrap_or(format!("Unnamed Device {}", device_num));
            device_names.push(device_name);
            devices.push(device);
        }
    }
    //returns true if we need to switch audio devices
    fn draw_device_list(context: &mut AppContext, ui: &Ui) -> bool {
        ui.window("Input Devices")
            .size(
                [
                    context.window_size_x as f32 / 2.0,
                    context.window_size_y as f32 / 2.0,
                ],
                imgui::Condition::FirstUseEver,
            )
            .resizable(true)
            .movable(true)
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .build(|| -> bool {
                let old_device_num = context.device_number;
                const NULL_STR: &str = "";
                let mut refs = [NULL_STR; 20];
                let mut len: usize = 0;
                context
                    .device_names
                    .iter()
                    .enumerate()
                    .for_each(|(i, name)| {
                        let ref_slot = refs.get_mut(i);
                        match ref_slot {
                            Some(slot) => {
                                *slot = name.as_ref();
                                len += 1;
                            }
                            None => {}
                        }
                    });
                if ui.list_box(
                    "Input Devices",
                    &mut context.device_number,
                    &refs[0..len],
                    7,
                ) && old_device_num != context.device_number
                {
                    return true;
                } else {
                    return false;
                }
            })
            .unwrap_or(false)
    }

    fn swap_device(
        current_stream: &mut Option<Stream>,
        devices: &mut Vec<Device>,
        device_number: i32,
        audio_analyzer: &Arc<Mutex<AudioAnalyzer>>,
        sample_buffer: &Arc<Mutex<CircularBuffer<f32>>>,
    ) -> anyhow::Result<()> {
        if current_stream.is_none() {
            let device = &devices[device_number as usize];
            let config = device
                .supported_input_configs()?
                .into_iter()
                .find(|config| config.sample_format().is_float());

            if config.is_none() {
                return Err(Error::msg("device does not support a float stream"));
            } else {
                let config = config.unwrap().with_max_sample_rate().config();
                let cloned_arc = sample_buffer.clone();
                let analyzer_arc = audio_analyzer.clone();
                let stream = device.build_input_stream(
                    &config,
                    move |a, _| {
                        Self::write_callback(a, &cloned_arc, &analyzer_arc);
                    },
                    move |_| {},
                    None,
                )?;
                let mut audio_analyzer = audio_analyzer.lock().unwrap();
                let mut sample_buffer = sample_buffer.lock().unwrap();
                let new_analyzer = AudioAnalyzer::new(
                    config.sample_rate.0,
                    1024 * 50,
                    3,
                    3,
                    440,
                    crate::audio_analysis::WindowType::Hann,
                );

                *audio_analyzer = new_analyzer;

                println!("sample rate: {:?}", config.sample_rate);
                stream.play()?;
                *current_stream = Some(stream);
            }
        }
        Ok(())
    }
}
