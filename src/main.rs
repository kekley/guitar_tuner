use anyhow::Ok;
use tuner::app::App;

fn main() -> anyhow::Result<()> {
    let app = App::new()
        .vsync_enabled(true)
        .window_height(720)
        .window_width(1280)
        .build()?;

    app.run()?;

    Ok(())
}
