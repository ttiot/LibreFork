use adw::prelude::*;
use gtk::Orientation;
use gtk4 as gtk;
use librefork_core::{CommitInfo, FileDiff};

#[derive(Clone)]
pub struct CommitDetails {
    root: gtk::Box,
    header: gtk::Label,
    meta: gtk::Label,
    message: gtk::TextView,
    diff: gtk::TextView,
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

        let diff = gtk::TextView::new();
        diff.set_editable(false);
        diff.set_cursor_visible(false);
        diff.set_monospace(true);

        root.append(&header);
        root.append(&meta);
        root.append(&message);
        root.append(&diff);

        Self {
            root,
            header,
            meta,
            message,
            diff,
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    pub fn clear(&self) {
        self.header.set_text("");
        self.meta.set_text("");
        self.message.buffer().set_text("");
        self.diff.buffer().set_text("");
    }

    pub fn show_commit(&self, info: &CommitInfo, message: &str, diffs: &[FileDiff]) {
        self.header
            .set_text(&format!("[{}] {}", info.short_id, info.summary));
        let refs = if info.refs.is_empty() {
            String::new()
        } else {
            format!(" • refs: {}", info.refs.join(", "))
        };
        self.meta.set_text(&format!(
            "{} <{}> • {} • {} parents{}",
            info.author, info.email, info.time, info.parents, refs
        ));
        self.message.buffer().set_text(message);

        let mut text = String::new();
        for file in diffs {
            text.push_str(&format!("diff -- {}\n", file.path));
            for line in &file.lines {
                let left = line.left.as_deref().unwrap_or("");
                let right = line.right.as_deref().unwrap_or("");
                text.push_str(&format!("{:<40} | {}\n", left, right));
            }
            text.push('\n');
        }
        self.diff.buffer().set_text(&text);
    }
}
