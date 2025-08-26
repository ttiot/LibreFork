
use adw::prelude::*;
use gtk4 as gtk;
use gtk::{Orientation};
use librefork_core::CommitInfo;

#[derive(Clone)]
pub struct CommitDetails {
    root: gtk::Box,
    header: gtk::Label,
    meta: gtk::Label,
    message: gtk::TextView,
}

impl CommitDetails {
    pub fn new() -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 8);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let header = gtk::Label::new(None);
        header.add_css_class("title-3");
        header.set_xalign(0.0);

        let meta = gtk::Label::new(None);
        meta.add_css_class("dim-label");
        meta.set_xalign(0.0);

        let message = gtk::TextView::new();
        message.set_editable(false);
        message.set_cursor_visible(false);
        message.set_monospace(true);
        message.set_wrap_mode(gtk::WrapMode::WordChar);

        root.append(&header);
        root.append(&meta);
        root.append(&message);

        Self { root, header, meta, message }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    pub fn clear(&self) {
        self.header.set_text("");
        self.meta.set_text("");
        self.message.buffer().set_text("");
    }

    pub fn show_commit(&self, info: &CommitInfo, message: &str) {
        self.header.set_text(&format!("[{}] {}", info.short_id, info.summary));
        let refs = if info.refs.is_empty() { String::new() } else { format!(" • refs: {}", info.refs.join(", ")) };
        self.meta.set_text(&format!("{} <{}> • {} • {} parents{}",
            info.author, info.email, info.time, info.parents, refs
        ));
        self.message.buffer().set_text(message);
    }
}
