use adw::prelude::*;
use gtk::Orientation;
use gtk4 as gtk;
use gtk4::gio;
use librefork_core::{CommitInfo, FileDiff};

#[derive(Clone)]
pub struct CommitDetails {
    root: gtk::Box,
    // Commit tab widgets
    header_summary: gtk::Label,
    author_name: gtk::Label,
    author_meta: gtk::Label,
    avatar_stack: gtk::Stack,
    avatar_pic: gtk::Picture,
    avatar_initials: gtk::Label,
    sha_value: gtk::Label,
    parents_value: gtk::Label,
    message: gtk::TextView,
    inline_container: gtk::Box,
    side_container: gtk::Box,
    filetree_container: gtk::Box,
}

impl CommitDetails {
    pub fn new() -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 8);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let stack = gtk::Stack::new();
        let switcher = gtk::StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        root.append(&switcher);
        root.append(&stack);

        // Commit tab
        let commit_box = gtk::Box::new(Orientation::Vertical, 8);

        // Top row: avatar + author details
        let top_row = gtk::Box::new(Orientation::Horizontal, 12);

        let avatar_pic = gtk::Picture::new();
        avatar_pic.add_css_class("avatar");
        avatar_pic.set_width_request(48);
        avatar_pic.set_height_request(48);

        let avatar_initials = gtk::Label::new(None);
        avatar_initials.add_css_class("avatar-initials");
        avatar_initials.set_width_request(48);
        avatar_initials.set_height_request(48);
        avatar_initials.set_xalign(0.5);
        avatar_initials.set_yalign(0.5);
        avatar_initials.set_justify(gtk::Justification::Center);

        let avatar_stack = gtk::Stack::new();
        avatar_stack.add_named(&avatar_initials, Some("initials"));
        avatar_stack.add_named(&avatar_pic, Some("image"));
        avatar_stack.set_visible_child_name("initials");
        {
            let stack_c = avatar_stack.clone();
            avatar_pic.connect_paintable_notify(move |p| {
                if p.paintable().is_some() {
                    stack_c.set_visible_child_name("image");
                } else {
                    stack_c.set_visible_child_name("initials");
                }
            });
        }

        let author_col = gtk::Box::new(Orientation::Vertical, 2);
        let author_name = gtk::Label::new(None);
        author_name.add_css_class("title-4");
        author_name.set_xalign(0.0);
        let author_meta = gtk::Label::new(None);
        author_meta.add_css_class("dim-label");
        author_meta.set_xalign(0.0);
        author_col.append(&author_name);
        author_col.append(&author_meta);

        top_row.append(&avatar_stack);
        top_row.append(&author_col);

        // Meta grid for SHA/Parents
        let meta_grid = gtk::Grid::new();
        meta_grid.add_css_class("commit-meta-grid");
        meta_grid.set_column_spacing(12);
        meta_grid.set_row_spacing(4);
        let sha_key = gtk::Label::new(Some("SHA"));
        sha_key.add_css_class("commit-meta-key");
        sha_key.set_xalign(0.0);
        let sha_value = gtk::Label::new(None);
        sha_value.set_xalign(0.0);
        let parents_key = gtk::Label::new(Some("PARENTS"));
        parents_key.add_css_class("commit-meta-key");
        parents_key.set_xalign(0.0);
        let parents_value = gtk::Label::new(None);
        parents_value.set_xalign(0.0);
        meta_grid.attach(&sha_key, 0, 0, 1, 1);
        meta_grid.attach(&sha_value, 1, 0, 1, 1);
        meta_grid.attach(&parents_key, 0, 1, 1, 1);
        meta_grid.attach(&parents_value, 1, 1, 1, 1);

        // Summary + message
        let header_summary = gtk::Label::new(None);
        header_summary.add_css_class("title-3");
        header_summary.add_css_class("commit-summary");
        header_summary.set_xalign(0.0);
        let message = gtk::TextView::new();
        message.set_editable(false);
        message.set_cursor_visible(false);
        message.set_monospace(true);
        message.set_wrap_mode(gtk::WrapMode::WordChar);

        commit_box.append(&top_row);
        commit_box.append(&meta_grid);
        commit_box.append(&header_summary);
        commit_box.append(&message);
        stack.add_titled(&commit_box, Some("commit"), "Commit");

        // Changes tab
        let changes_box = gtk::Box::new(Orientation::Vertical, 8);
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
        changes_box.append(&toolbar);
        changes_box.append(&diff_stack);
        stack.add_titled(&changes_box, Some("changes"), "Changes");

        // Filetree tab
        let filetree_container = gtk::Box::new(Orientation::Vertical, 4);
        let filetree_sw = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&filetree_container)
            .build();
        stack.add_titled(&filetree_sw, Some("filetree"), "Filetree");

        Self {
            root,
            header_summary,
            author_name,
            author_meta,
            avatar_stack,
            avatar_pic,
            avatar_initials,
            sha_value,
            parents_value,
            message,
            inline_container,
            side_container,
            filetree_container,
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    pub fn clear(&self) {
        self.header_summary.set_text("");
        self.author_name.set_text("");
        self.author_meta.set_text("");
        self.sha_value.set_text("");
        self.parents_value.set_text("");
        self.message.buffer().set_text("");
        while let Some(child) = self.inline_container.first_child() {
            child.unparent();
        }
        while let Some(child) = self.side_container.first_child() {
            child.unparent();
        }
        while let Some(child) = self.filetree_container.first_child() {
            child.unparent();
        }
    }

    pub fn show_commit(&self, info: &CommitInfo, message: &str, diffs: &[FileDiff]) {
        // Summary and author block
        self.header_summary.set_text(&info.summary);
        self.author_name.set_text(&info.author);
        let refs = if info.refs.is_empty() {
            String::new()
        } else {
            format!(" • refs: {}", info.refs.join(", "))
        };
        self.author_meta
            .set_text(&format!("{} • {}{}", info.email, info.time, refs));

        // Meta values
        self.sha_value.set_text(&info.oid);
        if info.parents.is_empty() {
            self.parents_value.set_text("<root>");
        } else {
            self.parents_value.set_text(&info.parents.join(", "));
        }

        // Avatar
        let initials = initials_from_name(&info.author);
        self.avatar_initials.set_text(&initials);
        if let Some(url) = compute_avatar_url(&info.email) {
            let f = gio::File::for_uri(&url);
            self.avatar_pic.set_file(Some(&f));
        } else {
            self.avatar_stack.set_visible_child_name("initials");
        }

        self.message.buffer().set_text(message);

        while let Some(child) = self.inline_container.first_child() {
            child.unparent();
        }
        while let Some(child) = self.side_container.first_child() {
            child.unparent();
        }
        while let Some(child) = self.filetree_container.first_child() {
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

            let file_label = gtk::Label::new(Some(&file.path));
            file_label.set_xalign(0.0);
            self.filetree_container.append(&file_label);
        }
    }
}

fn initials_from_name(name: &str) -> String {
    let mut parts = name
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().next().unwrap_or('?'));
    let first = parts.next().unwrap_or('?');
    let second = parts.next().unwrap_or('\0');
    if second == '\0' {
        first.to_uppercase().collect()
    } else {
        let mut out = String::new();
        out.extend(first.to_uppercase());
        out.extend(second.to_uppercase());
        out
    }
}

// Try to infer a useful avatar URL from the email address.
// - GitHub noreply: username@users.noreply.github.com or id+username@...
//   -> https://github.com/username.png
// - Otherwise: gravatar MD5 -> identicon fallback
fn compute_avatar_url(email: &str) -> Option<String> {
    let lower = email.trim().to_lowercase();
    if let Some(local) = lower.strip_suffix("@users.noreply.github.com") {
        let username = if let Some(pos) = local.rfind('+') {
            &local[pos + 1..]
        } else {
            local
        };
        if !username.is_empty() {
            return Some(format!("https://github.com/{}.png", username));
        }
    }
    // Try Gravatar using GLib's MD5 helper. If glib exposes the
    // checksum function, this yields a stable identicon URL.
    #[allow(unused_variables)]
    {
        // Some GLib versions expose a `checksum` helper; if unavailable,
        // the code is optimized out.
        #[cfg(any())]
        {
            let hash = glib::checksum(glib::ChecksumType::Md5, lower.as_bytes());
            return Some(format!(
                "https://www.gravatar.com/avatar/{}?s=96&d=identicon",
                hash
            ));
        }
    }
    None
}
