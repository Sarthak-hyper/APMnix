use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use adw::ApplicationWindow;
use gtk::{
    Box, Button, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow, Spinner, SearchEntry,
};
use std::sync::{Arc, Mutex};
use glib;

use crate::api::{Package, fetch_all_packages, search_packages, get_curated};

pub struct StoreWindow;

impl StoreWindow {
    pub fn new(app: &adw::Application, sudo_password: Arc<Mutex<String>>) {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("APMNix — VimuktiOS Store")
            .default_width(1000)
            .default_height(700)
            .build();

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .build();

        // ── Header ───────────────────────────────────────────
        let header = adw::HeaderBar::new();
        let title_label = Label::builder()
            .label("APMNix")
            .css_classes(["title"])
            .build();
        header.set_title_widget(Some(&title_label));
        root.append(&header);

        // ── Search Bar ───────────────────────────────────────
        let search_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(20)
            .margin_end(20)
            .build();

        let search_entry = SearchEntry::builder()
            .placeholder_text("Search 129,000+ packages...")
            .hexpand(true)
            .build();

        search_box.append(&search_entry);
        root.append(&search_box);

        // ── Status Label ─────────────────────────────────────
        let status_label = Label::builder()
            .label("Loading packages...")
            .css_classes(["dim-label"])
            .margin_start(20)
            .halign(gtk::Align::Start)
            .build();
        root.append(&status_label);

        // ── Spinner ──────────────────────────────────────────
        let spinner = Spinner::builder()
            .spinning(true)
            .margin_top(20)
            .build();
        root.append(&spinner);

        // ── Package List ─────────────────────────────────────
        let list_box = ListBox::builder()
            .css_classes(["boxed-list"])
            .margin_top(10)
            .margin_bottom(20)
            .margin_start(20)
            .margin_end(20)
            .build();

        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .child(&list_box)
            .build();

        root.append(&scrolled);
        window.set_content(Some(&root));
        window.present();

        // ── Load packages using MainContext ───────────────────
        let ctx = glib::MainContext::default();
        ctx.spawn_local(async move {
            let (sender, receiver) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let result = fetch_all_packages();
                let _ = sender.send(result);
            });

            // Wait for packages to load
            let packages = loop {
                if let Ok(result) = receiver.try_recv() {
                    break result;
                }
                glib::timeout_future(std::time::Duration::from_millis(100)).await;
            };

            spinner.set_spinning(false);
            spinner.set_visible(false);

            match packages {
                Ok(packages) => {
                    let packages = Arc::new(packages);

                    status_label.set_label(&format!(
                        "Showing curated packages ({} total available)",
                        packages.len()
                    ));

                    // Show curated by default
                    let curated = get_curated(&packages);
                    populate_list(&list_box, &curated, sudo_password.clone());

                    // Search handler
                    let list_box_s = list_box.clone();
                    let packages_s = packages.clone();
                    let sudo_s = sudo_password.clone();
                    let status_s = status_label.clone();

                    search_entry.connect_search_changed(move |entry| {
                        let query = entry.text().to_string();
                        let results = if query.is_empty() {
                            get_curated(&packages_s)
                        } else {
                            search_packages(&packages_s, &query)
                        };

                        status_s.set_label(&format!("{} packages found", results.len()));
                        populate_list(&list_box_s, &results, sudo_s.clone());
                    });
                }
                Err(e) => {
                    status_label.set_label(&format!("Failed to load: {}", e));
                }
            }
        });
    }
}

// ── Populate list ─────────────────────────────────────────────
fn populate_list(
    list_box: &ListBox,
    packages: &[Package],
    sudo_password: Arc<Mutex<String>>,
) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
    for pkg in packages {
        let row = build_package_row(pkg, sudo_password.clone());
        list_box.append(&row);
    }
}

// ── Build single package row ──────────────────────────────────
fn build_package_row(pkg: &Package, sudo_password: Arc<Mutex<String>>) -> ListBoxRow {
    let row = ListBoxRow::builder()
        .activatable(false)
        .build();

    let hbox = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();

    // ── Left: package info ────────────────────────────────────
    let info_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .build();

    let name_label = Label::builder()
        .label(&pkg.name)
        .css_classes(["heading"])
        .halign(gtk::Align::Start)
        .build();

    let desc_label = Label::builder()
        .label(&pkg.description)
        .css_classes(["dim-label", "caption"])
        .halign(gtk::Align::Start)
        .wrap(true)
        .max_width_chars(60)
        .build();

    let version_label = Label::builder()
        .label(&format!("v{}", pkg.version))
        .css_classes(["caption", "dim-label"])
        .halign(gtk::Align::Start)
        .build();

    info_box.append(&name_label);
    info_box.append(&desc_label);
    info_box.append(&version_label);

    // ── Right: buttons ────────────────────────────────────────
    let btn_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .valign(gtk::Align::Center)
        .build();

    let install_btn = Button::builder()
        .label("Install")
        .css_classes(["suggested-action", "pill"])
        .width_request(100)
        .build();

    let try_btn = Button::builder()
        .label("Try")
        .css_classes(["pill"])
        .width_request(100)
        .build();

    // ── Mark already installed packages ──────────────────────
    if crate::config::is_installed(&pkg.attribute) {
        install_btn.set_label("Installed ✓");
        install_btn.set_css_classes(&["success", "pill"]);
        install_btn.set_sensitive(false);
    }

    // ── Install button ────────────────────────────────────────
    let pkg_attr = pkg.attribute.clone();
    let pkg_name = pkg.name.clone();
    let sudo_clone = sudo_password.clone();
    let install_clone = install_btn.clone();
    let try_clone = try_btn.clone();

    install_btn.connect_clicked(move |_| {
        let attr = pkg_attr.clone();
        let name = pkg_name.clone();
        let password = sudo_clone.lock().unwrap().clone();
        let btn = install_clone.clone();
        let try_b = try_clone.clone();

        btn.set_sensitive(false);
        try_b.set_sensitive(false);
        btn.set_label("Installing...");

        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = install_package(&attr, &password);
            let _ = sender.send(result);
        });

        let ctx = glib::MainContext::default();
        ctx.spawn_local(async move {
            let result = loop {
                if let Ok(r) = receiver.try_recv() {
                    break r;
                }
                glib::timeout_future(std::time::Duration::from_millis(200)).await;
            };

            match result {
                Ok(_) => {
                    btn.set_label("Installed ✓");
                    btn.set_css_classes(&["success", "pill"]);
                    btn.set_sensitive(false);
                }
                Err(e) => {
    eprintln!("Install failed for {}: {}", name, e);
    // Show full error in a dialog
    let dialog = adw::MessageDialog::builder()
        .heading("Installation Failed")
        .body(&format!("{}", e))
        .build();
    dialog.add_response("ok", "OK");
    dialog.present();

    btn.set_label("Failed ✗");
    btn.set_css_classes(&["destructive-action", "pill"]);
    btn.set_sensitive(true);
}
            }
            try_b.set_sensitive(true);
        });
    });

    // ── Try button ────────────────────────────────────────────
    let pkg_attr_try = pkg.attribute.clone();
    let pkg_name_try = pkg.name.clone();

    try_btn.connect_clicked(move |btn| {
        let attr = pkg_attr_try.clone();
        let name = pkg_name_try.clone();
        btn.set_label("Launching...");
        btn.set_sensitive(false);
        let btn_clone = btn.clone();

        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = try_package(&attr);
            let _ = sender.send(result);
        });

        let ctx = glib::MainContext::default();
        ctx.spawn_local(async move {
            let result = loop {
                if let Ok(r) = receiver.try_recv() {
                    break r;
                }
                glib::timeout_future(std::time::Duration::from_millis(200)).await;
            };

            match result {
                Ok(_) => btn_clone.set_label("Try"),
                Err(e) => {
                    eprintln!("Try failed for {}: {}", name, e);
                    btn_clone.set_label("Try");
                }
            }
            btn_clone.set_sensitive(true);
        });
    });

    btn_box.append(&install_btn);
    btn_box.append(&try_btn);
    hbox.append(&info_box);
    hbox.append(&btn_box);
    row.set_child(Some(&hbox));
    row
}

// ── Install package ───────────────────────────────────────────
fn install_package(attribute: &str, sudo_password: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    use std::io::Write;

    // First verify sudo works
    let mut check = Command::new("sudo")
        .args(["-S", "true"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = check.stdin.as_mut() {
        stdin.write_all(format!("{}\n", sudo_password).as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let check_output = check.wait_with_output().map_err(|e| e.to_string())?;
    if !check_output.status.success() {
        return Err("Sudo authentication failed".to_string());
    }

    // Backup
    crate::config::backup_config()?;

    // Add to configuration.nix
    crate::config::add_package(attribute)?;

    // nixos-rebuild switch
    let mut child = Command::new("sudo")
        .args(["-S", "nixos-rebuild", "switch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(format!("{}\n", sudo_password).as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let _ = crate::config::restore_backup();
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
// ── Try package ───────────────────────────────────────────────
fn try_package(attribute: &str) -> Result<(), String> {
    std::process::Command::new("nix-shell")
        .args(["-p", attribute])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
