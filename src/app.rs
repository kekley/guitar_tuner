use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, Ok};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, FromSample, Sample, SampleFormat, SampleRate, Stream, StreamConfig,
    ALL_HOSTS,
};
use imgui::Context;
use imgui_glow_renderer::{
    glow::{self, HasContext, COLOR_BUFFER_BIT},
    AutoRenderer,
};
use imgui_sdl2_support::SdlPlatform;
use sdl2::{
    event::{self, Event},
    video::{gl_attr::GLAttr, GLContext, Window},
    EventPump, Sdl, VideoSubsystem,
};

use crate::circular_buffer::CircularBuffer;

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

        let app = App {
            sdl,
            video_subsystem,
            window,
            gl_context,
            imgui_context,
            imgui_platform,
            imgui_renderer,
            event_pump,
        };

        Ok(app)
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
}

impl App {
    pub fn new() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        let App {
            mut sdl,
            mut video_subsystem,
            mut window,
            mut gl_context,
            mut imgui_context,
            mut imgui_platform,
            mut imgui_renderer,
            mut event_pump,
        } = self;
        let (window_size_x, window_size_y) = window.size();
        let window_size_x = window_size_x as f32;
        let window_size_y = window_size_y as f32;
        let host = cpal::default_host();

        let devices = host.input_devices()?.collect::<Vec<_>>();
        let device_names = devices
            .iter()
            .map(|f| f.name().unwrap())
            .collect::<Vec<_>>();
        let name_refs = device_names.iter().map(|f| f).collect::<Vec<_>>();

        let mut device_number: i32 = 0;
        let mut current_stream: Option<Stream> = None;
        let sample_buffer = Arc::new(Mutex::new(CircularBuffer::<f32>::new(BUFFER_SIZE)));

        'main: loop {
            if current_stream.is_none() {
                println!("huh");
                let device = &devices[device_number as usize];
                let config = device
                    .supported_input_configs()?
                    .into_iter()
                    .find(|config| config.sample_format().is_float());

                if config.is_none() {
                    println!("device {} does not support a float stream", device_number);
                    device_number += 1;
                    continue;
                } else {
                    let buffer_arc_clone = sample_buffer.clone();
                    let config = config.unwrap().with_max_sample_rate().config();
                    let stream = device.build_input_stream(
                        &config,
                        move |a, _| {
                            Self::write_input_data(a, &buffer_arc_clone);
                        },
                        move |a| {},
                        None,
                    )?;
                    println!("sample rate: {:?}", config.sample_rate);
                    stream.play()?;
                    current_stream = Some(stream);
                }
            }
            for event in event_pump.poll_iter() {
                //event passed to imgui
                imgui_platform.handle_event(&mut imgui_context, &event);

                if let Event::Quit { .. } = event {
                    break 'main;
                }
            }

            imgui_platform.prepare_frame(&mut imgui_context, &window, &event_pump);

            let ui = imgui_context.new_frame();
            let lock = sample_buffer.clone();
            let mut lock = lock.lock().unwrap();
            ///////////////////////////////////////////////
            //ui code  goes here
            ui.window("Input Devices")
                .size(
                    [window_size_x / 2.0, window_size_y / 2.0],
                    imgui::Condition::Always,
                )
                .resizable(false)
                .position([0.0, 0.0], imgui::Condition::Always)
                .build(|| {
                    let slice = lock.make_contiguous();
                    println!("slice len: {}", slice.len());
                    if ui.list_box("list box", &mut device_number, &name_refs, 7) {
                        current_stream = None;
                    }

                    let a = ui
                        .plot_lines("Sample Data:", &slice)
                        .scale_max(0.5)
                        .scale_min(-0.5)
                        .graph_size([500.0, 200.0]);
                    a.build();
                });

            //////////////////////////////////////////////

            let draw_data = imgui_context.render();

            unsafe {
                imgui_renderer.gl_context().clear(COLOR_BUFFER_BIT);
            }

            imgui_renderer
                .render(draw_data)
                .map_err(|error| anyhow!(error))?;

            window.gl_swap_window();
            drop(lock);
        }
        Ok(())
    }
    fn write_input_data(input: &[f32], buffer: &Arc<Mutex<CircularBuffer<f32>>>) {
        let mut buffer = buffer.lock().unwrap();
        (0..input.len()).for_each(|i| {
            let _ = buffer.push_back(input[i]);
        });
    }
}
