use std::task::Context;

use anyhow::Ok;
use imgui::Context;
use imgui_glow_renderer::glow;
use sdl2::{
    video::{GLContext, Window},
    Sdl, VideoSubsystem,
};

pub const WINDOW_TITLE: &str = "dodge left dodge right";
pub const DEFAULT_WIDTH: usize = 800;
pub const DEFAULT_HEIGHT: usize = 600;

unsafe fn get_glow_context(window: &Window) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function(|s| window.subsystem().gl_get_proc_address(s) as _)
    }
}

fn init_app() -> anyhow::Result<()> {
    let sdl = initialize_sdl()?;
    let window = create_window(sdl_video)?;

    let gl_context = create_opengl_context(&window)?;

    //enable v-sync
    window.subsystem().gl_set_swap_interval(1)?;

    let glow = unsafe { get_glow_context(&window) };

    let imgui = init_imgui();
    todo!()
}

fn initialize_sdl() -> anyhow::Result<Sdl> {
    let sdl = sdl2::init()?;
    let video_subsystem = sdl.video()?;

    let gl_attr = video_subsystem.gl_attr();

    //opengl 3.3
    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
}

fn create_window(sdl_video: &VideoSubsystem) -> anyhow::Result<Window> {
    video_subsystem
        .window(WINDOW_TITLE, DEFAULT_WIDTH, DEFAULT_HEIGHT)
        .allow_highdpi()
        .opengl()
        .position_centered()
        .resizable()
        .build()?
}

fn create_opengl_context(window: &Window) -> anyhow::Result<GLContext> {
    let context = window.gl_create_context()?;
    window.gl_make_current(&context);
    Ok(context)
}

fn init_imgui() -> anyhow::Result<Context> {
    let mut imgui = imgui::Context::create();

    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);

    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    imgui
}
