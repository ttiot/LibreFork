use gtk::prelude::*;
use gtk::Orientation;
use gtk4 as gtk;

#[derive(Clone)]
pub struct HomePanel {
    root: gtk::Box,
}

impl HomePanel {
    pub fn new() -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 12);
        root.add_css_class("home-container");
        root.set_hexpand(true);
        root.set_vexpand(true);

        // Fake logo: gradient circle with "LF" initials
        let logo = gtk::Box::new(Orientation::Vertical, 0);
        logo.add_css_class("home-logo");
        let logo_lbl = gtk::Label::new(Some("LF"));
        logo_lbl.add_css_class("home-logo-text");
        logo.set_halign(gtk::Align::Center);
        logo.set_valign(gtk::Align::Center);
        logo.append(&logo_lbl);

        let title = gtk::Label::new(Some("LibreFork"));
        title.add_css_class("home-title");
        title.set_halign(gtk::Align::Center);

        // From cargo env at build-time
        let author = option_env!("CARGO_PKG_AUTHORS").unwrap_or("LibreFork Team");
        let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.0");
        let subtitle = gtk::Label::new(Some(&format!("Auteur: {}", author)));
        subtitle.add_css_class("home-subtitle");
        subtitle.set_halign(gtk::Align::Center);

        let version_lbl = gtk::Label::new(Some(&format!("Version: {}", version)));
        version_lbl.add_css_class("home-subtitle");
        version_lbl.set_halign(gtk::Align::Center);

        root.append(&logo);
        root.append(&title);
        root.append(&subtitle);
        root.append(&version_lbl);

        Self { root }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}

