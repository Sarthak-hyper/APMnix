mod api;
mod store;
mod config;  // we will add this next

// Replace launch_store function with:

use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use adw::{Application, ApplicationWindow};
use gtk::{
    Box, Button, Image, Label, Orientation, PasswordEntry,
};
use std::process::Command;
use std::sync::{Arc, Mutex};

const APP_ID: &str = "org.vimukti.store";

fn main() {
    println!("Fetching packages from nixpkgs...");
    match api::fetch_all_packages() {
        Ok(packages) => {
            println!("Total packages fetched: {}", packages.len());
            let results = api::search_packages(&packages, "firefox");
            for pkg in results.iter().take(3) {
                println!("Found: {} v{} - {}", pkg.name, pkg.version, pkg.description);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    let app = Application::builder()
        .application_id(APP_ID)
        .build();
    app.connect_activate(build_password_dialog);
    app.run();
}

fn build_password_dialog(app: &Application) {
    // Shared sudo password across app
    let sudo_password: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("APMNix")
        .default_width(400)
        .default_height(300)
        .resizable(false)
        .build();

    // Main container
    let container = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(20)
        .margin_top(40)
        .margin_bottom(40)
        .margin_start(40)
        .margin_end(40)
        .build();

    // Icon
    let icon = Image::builder()
        .icon_name("system-software-install")
        .pixel_size(64)
        .build();

    // Title
    let title = Label::builder()
        .label("APMNix")
        .css_classes(["title-1"])
        .build();

    // Subtitle
    let subtitle = Label::builder()
        .label("Administrator access is required to install packages")
        .css_classes(["dim-label"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    // Password entry
    let password_entry = PasswordEntry::builder()
        .placeholder_text("Enter your password...")
        .show_peek_icon(true)
        .build();

    // Error label (hidden by default)
    let error_label = Label::builder()
        .label("")
        .css_classes(["error"])
        .visible(false)
        .build();

    // Authenticate button
    let auth_button = Button::builder()
        .label("Authenticate")
        .css_classes(["suggested-action", "pill"])
        .build();

    // Clone refs for button click
    let password_entry_clone = password_entry.clone();
    let error_label_clone = error_label.clone();
    let window_clone = window.clone();
    let app_clone = app.clone();
    let sudo_password_clone = sudo_password.clone();

    // Authenticate on button click
    auth_button.connect_clicked(move |_| {
        let password = password_entry_clone.text().to_string();

        if password.is_empty() {
            error_label_clone.set_label("Password cannot be empty");
            error_label_clone.set_visible(true);
            return;
        }

        if validate_password(&password) {
            // Store password
            *sudo_password_clone.lock().unwrap() = password.clone();

            // Keep sudo session alive
            keep_sudo_alive(password.clone());

            // Close password dialog and launch store
            window_clone.close();
            launch_store(&app_clone, sudo_password_clone.clone());
        } else {
            error_label_clone.set_label("Incorrect password, please try again");
            error_label_clone.set_visible(true);
            password_entry_clone.set_text("");
        }
    });

    // Also authenticate on Enter key
    let password_entry_clone2 = password_entry.clone();
    let auth_button_clone = auth_button.clone();
    password_entry.connect_activate(move |_| {
        let _ = password_entry_clone2.text();
        auth_button_clone.emit_clicked();
    });

    // Pack everything into container
    container.append(&icon);
    container.append(&title);
    container.append(&subtitle);
    container.append(&password_entry);
    container.append(&error_label);
    container.append(&auth_button);

    window.set_content(Some(&container));
    window.present();
}

fn validate_password(password: &str) -> bool {
    let output = Command::new("sudo")
        .args(["-S", "true"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match output {
        Ok(mut child) => {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(format!("{}\n", password).as_bytes());
            }
            let status = child.wait().expect("Failed to wait on sudo");
            status.success()
        }
        Err(_) => false,
    }
}

fn keep_sudo_alive(password: String) {
    std::thread::spawn(move || {
        loop {
            // Refresh sudo timestamp every 4 minutes
            std::thread::sleep(std::time::Duration::from_secs(240));

            let mut child = Command::new("sudo")
                .args(["-S", "-v"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to refresh sudo");

            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(format!("{}\n", password).as_bytes());
            }
            let _ = child.wait();
        }
    });
}

// Replace launch_store function with:
fn launch_store(app: &Application, sudo_password: Arc<Mutex<String>>) {
    store::StoreWindow::new(app, sudo_password);
}
