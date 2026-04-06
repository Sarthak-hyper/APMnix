mod api;
mod store;
mod config;

use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use adw::Application;

const APP_ID: &str = "org.vimukti.store";

fn main() {
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    // Launch directly into the store — no password gate at startup.
    // Sudo is only requested when the user explicitly clicks "Install (System)".
    app.connect_activate(|app| {
        store::StoreWindow::new(app);
    });

    app.run();
}
