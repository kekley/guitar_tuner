use anyhow::Ok;
use tuner::app::App;

fn main() -> anyhow::Result<()> {
    let app = App::new()
        .vsync_enabled(true)
        .window_height(900)
        .window_width(1600)
        .window_title("jorkin it")
        .build()?;

    app.run()?;

    Ok(())
}
