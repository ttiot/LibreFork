mod app;
mod widgets;
mod starred;
mod recents;

use adw::prelude::*;
use adw::Application;

fn main() {
    // Requis par libadwaita
    adw::init().expect("Failed to init libadwaita");

    let app = Application::builder()
        .application_id("dev.librefork.app")
        .build();

    app.connect_activate(|app| {
        app::build_ui(app);
    });

    app.run();
}
