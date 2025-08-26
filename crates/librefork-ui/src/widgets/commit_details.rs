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
    toolbar: gtk::Box,
    diff_stack: gtk::Stack,
    inline_container: gtk::Box,
    side_container: gtk::Box,
    inline_button: gtk::ToggleButton,
    side_button: gtk::ToggleButton,
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

        let toolbar = gtk::Box::new(Orientation::Horizontal, 4);
        let inline_button = gtk::ToggleButton::builder()
            .icon_name("view-list-symbolic")
            .build();
        let side_button = gtk::ToggleButton::builder()
            .icon_name("view-dual-symbolic")
            .group(&inline_button)
            .build();
        side_button.set_active(true);
        toolbar.append(&inline_button);
        toolbar.append(&side_button);

        let diff_stack = gtk::Stack::new();
        let inline_container = gtk::Box::new(Orientation::Vertical, 8);
        let inline_sw = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&inline_container)
            .build();
        let side_container = gtk::Box::new(Orientation::Vertical, 8);
        let side_sw = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&side_container)
            .build();
        diff_stack.add_named(&inline_sw, Some("inline"));
        diff_stack.add_named(&side_sw, Some("side"));
        diff_stack.set_visible_child_name("side");

        {
            let stack = diff_stack.clone();
            side_button.connect_toggled(move |btn| {
                if btn.is_active() {
                    stack.set_visible_child_name("side");
                }
            });
        }
        {
            let stack = diff_stack.clone();
            inline_button.connect_toggled(move |btn| {
                if btn.is_active() {
                    stack.set_visible_child_name("inline");
                }
            });
        }

        root.append(&header);
        root.append(&meta);
        root.append(&message);
        root.append(&toolbar);
        root.append(&diff_stack);

        Self {
            root,
            header,
            meta,
            message,
            toolbar,
            diff_stack,
            inline_container,
            side_container,
            inline_button,
            side_button,
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    pub fn clear(&self) {
        self.header.set_text("");
        self.meta.set_text("");
        self.message.buffer().set_text("");
        while let Some(child) = self.inline_container.first_child() {
            child.unparent();
        }
        while let Some(child) = self.side_container.first_child() {
            child.unparent();
        }
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

        while let Some(child) = self.inline_container.first_child() {
            child.unparent();
        }
        while let Some(child) = self.side_container.first_child() {
            child.unparent();
        }

        for file in diffs {
            let title = format!("{} ({})", file.path, file.status);
            let frame_inline = gtk::Frame::new(Some(&title));
            let view_inline = gtk::TextView::new();
            view_inline.set_editable(false);
            view_inline.set_cursor_visible(false);
            view_inline.set_monospace(true);
            let buffer_i = view_inline.buffer();
            let tag_add_i = buffer_i
                .create_tag(Some("add"), &[("foreground", &"green")])
                .unwrap();
            let tag_del_i = buffer_i
                .create_tag(Some("del"), &[("foreground", &"red")])
                .unwrap();
            let mut iter_i = buffer_i.end_iter();
            for line in &file.lines {
                match (line.left.as_ref(), line.right.as_ref()) {
                    (Some(l), None) => {
                        buffer_i.insert_with_tags(&mut iter_i, &format!("-{}\n", l), &[&tag_del_i]);
                    }
                    (None, Some(r)) => {
                        buffer_i.insert_with_tags(&mut iter_i, &format!("+{}\n", r), &[&tag_add_i]);
                    }
                    (Some(l), Some(r)) => {
                        if l == r {
                            buffer_i.insert(&mut iter_i, &format!(" {}\n", l));
                        } else {
                            buffer_i.insert_with_tags(
                                &mut iter_i,
                                &format!("-{}\n", l),
                                &[&tag_del_i],
                            );
                            buffer_i.insert_with_tags(
                                &mut iter_i,
                                &format!("+{}\n", r),
                                &[&tag_add_i],
                            );
                        }
                    }
                    (None, None) => (),
                }
            }
            frame_inline.set_child(Some(&view_inline));
            self.inline_container.append(&frame_inline);

            let frame_side = gtk::Frame::new(Some(&title));
            let view_side = gtk::TextView::new();
            view_side.set_editable(false);
            view_side.set_cursor_visible(false);
            view_side.set_monospace(true);
            let buffer_s = view_side.buffer();
            let tag_add_s = buffer_s
                .create_tag(Some("add"), &[("foreground", &"green")])
                .unwrap();
            let tag_del_s = buffer_s
                .create_tag(Some("del"), &[("foreground", &"red")])
                .unwrap();
            let mut iter_s = buffer_s.end_iter();
            for line in &file.lines {
                match (line.left.as_ref(), line.right.as_ref()) {
                    (Some(l), None) => {
                        buffer_s.insert_with_tags(
                            &mut iter_s,
                            &format!("{:<40}", l),
                            &[&tag_del_s],
                        );
                        buffer_s.insert(&mut iter_s, " | \n");
                    }
                    (None, Some(r)) => {
                        buffer_s.insert(&mut iter_s, &format!("{:<40}", ""));
                        buffer_s.insert(&mut iter_s, " | ");
                        buffer_s.insert_with_tags(&mut iter_s, &format!("{}\n", r), &[&tag_add_s]);
                    }
                    (Some(l), Some(r)) => {
                        if l == r {
                            buffer_s.insert(&mut iter_s, &format!("{:<40} | {}\n", l, r));
                        } else {
                            buffer_s.insert_with_tags(
                                &mut iter_s,
                                &format!("{:<40}", l),
                                &[&tag_del_s],
                            );
                            buffer_s.insert(&mut iter_s, " | ");
                            buffer_s.insert_with_tags(
                                &mut iter_s,
                                &format!("{}\n", r),
                                &[&tag_add_s],
                            );
                        }
                    }
                    (None, None) => buffer_s.insert(&mut iter_s, "\n"),
                }
            }
            frame_side.set_child(Some(&view_side));
            self.side_container.append(&frame_side);
        }
    }
}
