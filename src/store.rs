use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use adw::ApplicationWindow;
use gtk::{
    Box, Button, Entry, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow, Spinner, SearchEntry, ToggleButton,
};
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use glib;

use crate::api::{Package, fetch_all_packages, search_packages, get_curated};

#[derive(Clone, Copy, PartialEq)]
pub enum InstallMode {
    User,
    System,
}

pub struct StoreWindow;

impl StoreWindow {
    pub fn new(app: &adw::Application) {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("APMNix — VimuktiOS Store")
            .default_width(1000)
            .default_height(700)
            .build();

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .build();

        // ── Header & Mode Switcher ───────────────────────────
        let header = adw::HeaderBar::new();

        let switcher_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["linked"])
            .valign(gtk::Align::Center)
            .build();

        let user_mode_btn = ToggleButton::builder()
            .label("User (home.nix)")
            .active(true)
            .build();

        let sys_mode_btn = ToggleButton::builder()
            .label("System (configuration.nix)")
            .build();

        sys_mode_btn.set_group(Some(&user_mode_btn));

        switcher_box.append(&user_mode_btn);
        switcher_box.append(&sys_mode_btn);
        header.set_title_widget(Some(&switcher_box));
        root.append(&header);

        // State tracking
        let current_mode = Rc::new(RefCell::new(InstallMode::User));
        let current_query = Rc::new(RefCell::new(String::new()));

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

        // ── Load packages ─────────────────────────────────────
        let ctx = glib::MainContext::default();
        ctx.spawn_local(async move {
            let (sender, receiver) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let result = fetch_all_packages();
                let _ = sender.send(result);
            });

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

                    // Initial populate
                    let curated = get_curated(&packages);
                    populate_list(&list_box, &curated, *current_mode.borrow());

                    // ── Connect Mode Switcher ─────────────────
                    let list_box_m = list_box.clone();
                    let packages_m = packages.clone();
                    let query_m = current_query.clone();
                    let mode_m = current_mode.clone();

                    user_mode_btn.connect_toggled(move |btn| {
                        let new_mode = if btn.is_active() {
                            InstallMode::User
                        } else {
                            InstallMode::System
                        };
                        *mode_m.borrow_mut() = new_mode;

                        let query = query_m.borrow().clone();
                        let results = if query.is_empty() {
                            get_curated(&packages_m)
                        } else {
                            search_packages(&packages_m, &query)
                        };
                        populate_list(&list_box_m, &results, new_mode);
                    });

                    // ── Connect Search Entry ──────────────────
                    let list_box_s = list_box.clone();
                    let packages_s = packages.clone();
                    let query_s = current_query.clone();
                    let mode_s = current_mode.clone();
                    let status_s = status_label.clone();

                    search_entry.connect_search_changed(move |entry| {
                        let query = entry.text().to_string();
                        *query_s.borrow_mut() = query.clone();

                        let results = if query.is_empty() {
                            get_curated(&packages_s)
                        } else {
                            search_packages(&packages_s, &query)
                        };
                        status_s.set_label(&format!("{} packages found", results.len()));
                        populate_list(&list_box_s, &results, *mode_s.borrow());
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
fn populate_list(list_box: &ListBox, packages: &[Package], mode: InstallMode) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
    for pkg in packages {
        let row = build_package_row(pkg, mode);
        list_box.append(&row);
    }
}

// ── Build single package row ──────────────────────────────────
fn build_package_row(pkg: &Package, mode: InstallMode) -> ListBoxRow {
    let row = ListBoxRow::builder().activatable(false).build();

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

    let attr = pkg.attribute.clone();
    let name = pkg.name.clone();

    let install_btn = Button::builder()
        .css_classes(["pill"])
        .width_request(140)
        .build();

    // ── Install button logic ──────────────────────────────────
    match mode {
        InstallMode::User => {
            if crate::config::is_installed_user(&attr) {
                install_btn.set_label("Installed ✓");
                install_btn.set_css_classes(&["success", "pill"]);
                install_btn.set_sensitive(false);
            } else {
                install_btn.set_label("Install (User)");
                install_btn.set_css_classes(&["suggested-action", "pill"]);

                let btn = install_btn.clone();
                let a = attr.clone();
                let n = name.clone();

                install_btn.connect_clicked(move |_| {
                    btn.set_label("Installing...");
                    btn.set_sensitive(false);

                    let (tx, rx) = std::sync::mpsc::channel();
                    let a2 = a.clone();
                    std::thread::spawn(move || {
                        let _ = tx.send(crate::config::add_package_user(&a2));
                    });

                    let ctx = glib::MainContext::default();
                    let btn_async = btn.clone();
                    let n_clone = n.clone();

                    ctx.spawn_local(async move {
                        let result = loop {
                            if let Ok(r) = rx.try_recv() {
                                break r;
                            }
                            glib::timeout_future(std::time::Duration::from_millis(200)).await;
                        };
                        match result {
                            Ok(_) => {
                                btn_async.set_label("Installed ✓");
                                btn_async.set_css_classes(&["success", "pill"]);
                            }
                            Err(e) => {
                                eprintln!("User install failed for {}: {}", n_clone, e);
                                show_error_dialog("Installation Failed", &e);
                                btn_async.set_label("Install (User)");
                                btn_async.set_css_classes(&["suggested-action", "pill"]);
                                btn_async.set_sensitive(true);
                            }
                        }
                    });
                });
            }
        }
        InstallMode::System => {
            if crate::config::is_installed_system(&attr) {
                install_btn.set_label("System ✓");
                install_btn.set_css_classes(&["success", "pill"]);
                install_btn.set_sensitive(false);
            } else {
                install_btn.set_label("Install (System)");
                install_btn.set_css_classes(&["destructive-action", "pill"]);

                let btn = install_btn.clone();
                let a = attr.clone();
                let n = name.clone();

                install_btn.connect_clicked(move |_| {
                    show_password_dialog_for_system(a.clone(), n.clone(), btn.clone());
                });
            }
        }
    }

    // ── Try button ────────────────────────────────────────────
    let try_btn = Button::builder()
        .label("Try (nix-shell)")
        .css_classes(["pill"])
        .width_request(140)
        .build();

    {
        let try_b = try_btn.clone();
        let a = attr.clone();
        let n = name.clone();

        try_btn.connect_clicked(move |_| {
            try_b.set_label("Launching...");
            try_b.set_sensitive(false);

            let (tx, rx) = std::sync::mpsc::channel();
            let attr_clone = a.clone();
            std::thread::spawn(move || {
                let result = std::process::Command::new("nix-shell")
                    .args(["-p", &attr_clone])
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string());
                let _ = tx.send(result);
            });

            let ctx = glib::MainContext::default();
            let try_b_async = try_b.clone();
            let n_async = n.clone();

            ctx.spawn_local(async move {
                let result = loop {
                    if let Ok(r) = rx.try_recv() {
                        break r;
                    }
                    glib::timeout_future(std::time::Duration::from_millis(200)).await;
                };
                if let Err(e) = result {
                    eprintln!("Try failed for {}: {}", n_async, e);
                }
                try_b_async.set_label("Try (nix-shell)");
                try_b_async.set_sensitive(true);
            });
        });
    }

    // ── Remove button ─────────────────────────────────────────
    let remove_btn = Button::builder()
        .label("Remove 🗑")
        .css_classes(["destructive-action", "pill"])
        .width_request(140)
        .build();

    // Only show remove if installed
    let is_installed = match mode {
        InstallMode::User => crate::config::is_installed_user(&attr),
        InstallMode::System => crate::config::is_installed_system(&attr),
    };
    remove_btn.set_visible(is_installed);

    {
        let rm_btn = remove_btn.clone();
        let install_btn_clone = install_btn.clone();
        let a = attr.clone();
        let n = name.clone();

        remove_btn.connect_clicked(move |_| {
            let attr_rm = a.clone();
            let name_rm = n.clone();
            let btn_rm = rm_btn.clone();
            let install_restore = install_btn_clone.clone();

            // Confirm dialog
            let dialog = adw::MessageDialog::builder()
                .heading("Remove Package")
                .body(&format!("Remove {} from system?", name_rm))
                .build();

            dialog.add_response("cancel", "Cancel");
            dialog.add_response("remove", "Remove");
            dialog.set_response_appearance(
                "remove",
                adw::ResponseAppearance::Destructive,
            );

            let btn2 = btn_rm.clone();
            let install2 = install_restore.clone();
            let a2 = attr_rm.clone();
            let n2 = name_rm.clone();
            let mode2 = mode;

            dialog.connect_response(None, move |_, response| {
                if response != "remove" {
                    return;
                }

                btn2.set_label("Removing...");
                btn2.set_sensitive(false);

                let a3 = a2.clone();
                let n3 = n2.clone();
                let btn3 = btn2.clone();
                let install3 = install2.clone();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let result = match mode2 {
                        InstallMode::User => crate::config::remove_package_user(&a3),
                        InstallMode::System => {
                            // System remove needs password
                            // For now use empty string — will prompt separately
                            crate::config::remove_package_system(&a3, "")
                        }
                    };
                    let _ = tx.send(result);
                });

                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let result = loop {
                        if let Ok(r) = rx.try_recv() {
                            break r;
                        }
                        glib::timeout_future(std::time::Duration::from_millis(200)).await;
                    };

                    match result {
                        Ok(_) => {
                            btn3.set_visible(false);
                            install3.set_label(match mode2 {
                                InstallMode::User => "Install (User)",
                                InstallMode::System => "Install (System)",
                            });
                            install3.set_css_classes(match mode2 {
                                InstallMode::User => &["suggested-action", "pill"],
                                InstallMode::System => &["destructive-action", "pill"],
                            });
                            install3.set_sensitive(true);
                        }
                        Err(e) => {
                            eprintln!("Remove failed for {}: {}", n3, e);
                            show_error_dialog("Remove Failed", &e);
                            btn3.set_label("Remove 🗑");
                            btn3.set_sensitive(true);
                        }
                    }
                });
            });

            dialog.present();
        });
    }

    btn_box.append(&install_btn);
    btn_box.append(&try_btn);
    btn_box.append(&remove_btn);
    hbox.append(&info_box);
    hbox.append(&btn_box);
    row.set_child(Some(&hbox));
    row
}

// ── Inline password dialog for system install ─────────────────
fn show_password_dialog_for_system(
    attribute: String,
    pkg_name: String,
    install_btn: Button,
) {
    let dialog = adw::MessageDialog::builder()
        .heading("System Installation")
        .body(&format!(
            "Installing <b>{}</b> system-wide requires your sudo password.\n\
             Leave blank to install for your user only instead.",
            pkg_name
        ))
        .body_use_markup(true)
        .build();

    let password_entry = Entry::builder()
        .placeholder_text("sudo password")
        .visibility(false)
        .input_purpose(gtk::InputPurpose::Password)
        .build();

    dialog.set_extra_child(Some(&password_entry));
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("install", "Install");
    dialog.set_response_appearance("install", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("install"));
    dialog.set_close_response("cancel");

    let attr = Arc::new(attribute);
    let name = Arc::new(pkg_name);
    let btn = install_btn.clone();
    let pwd_entry = password_entry.clone();

    dialog.connect_response(None, move |_dialog, response| {
        if response != "install" {
            return;
        }

        let password = pwd_entry.text().to_string();
        btn.set_sensitive(false);

        // Fallback to User Install if left blank
        if password.is_empty() {
            btn.set_label("Installing (User)...");
            btn.set_css_classes(&["suggested-action", "pill"]);

            let a = attr.as_str().to_string();
            let n = name.as_str().to_string();
            let b = btn.clone();

            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(crate::config::add_package_user(&a));
            });

            let ctx = glib::MainContext::default();
            let b_async = b.clone();
            let n_async = n.clone();

            ctx.spawn_local(async move {
                let result = loop {
                    if let Ok(r) = rx.try_recv() {
                        break r;
                    }
                    glib::timeout_future(std::time::Duration::from_millis(200)).await;
                };
                match result {
                    Ok(_) => {
                        b_async.set_label("Installed ✓");
                        b_async.set_css_classes(&["success", "pill"]);
                    }
                    Err(e) => {
                        eprintln!("Fallback user install failed for {}: {}", n_async, e);
                        show_error_dialog("Installation Failed", &e);
                        b_async.set_label("Install (System)");
                        b_async.set_css_classes(&["destructive-action", "pill"]);
                        b_async.set_sensitive(true);
                    }
                }
            });
            return;
        }

        // Standard System Install
        btn.set_label("Installing...");
        let a = attr.as_str().to_string();
        let n = name.as_str().to_string();
        let b = btn.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(crate::config::add_package_system(&a, &password));
        });

        let ctx = glib::MainContext::default();
        let b_async = b.clone();
        let n_async = n.clone();

        ctx.spawn_local(async move {
            let result = loop {
                if let Ok(r) = rx.try_recv() {
                    break r;
                }
                glib::timeout_future(std::time::Duration::from_millis(200)).await;
            };
            match result {
                Ok(_) => {
                    b_async.set_label("System ✓");
                    b_async.set_css_classes(&["success", "pill"]);
                }
                Err(e) => {
                    eprintln!("System install failed for {}: {}", n_async, e);
                    show_error_dialog("System Installation Failed", &e);
                    b_async.set_label("Install (System)");
                    b_async.set_css_classes(&["destructive-action", "pill"]);
                    b_async.set_sensitive(true);
                }
            }
        });
    });

    dialog.present();
}

// ── Utility ───────────────────────────────────────────────────
fn show_error_dialog(heading: &str, body: &str) {
    let dialog = adw::MessageDialog::builder()
        .heading(heading)
        .body(body)
        .build();
    dialog.add_response("ok", "OK");
    dialog.present();
}
