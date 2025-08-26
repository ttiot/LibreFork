
use adw::prelude::*;
use gtk4 as gtk;
use gtk4::pango;
use gtk::Orientation;
use std::cell::RefCell;
use std::rc::Rc;
use librefork_core::CommitInfo;

#[derive(Clone)]
pub struct CommitList {
    root: gtk::Box,
    list: gtk::ListBox,
    on_select: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
}

impl CommitList {
    pub fn new() -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 0);
        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::Single);

        root.append(&list);

        let on_select: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));

        let cb = on_select.clone();
        list.connect_row_selected(move |_, row| {
            if let (Some(row), Some(cb)) = (row, &*cb.borrow()) {
                let oid = row.widget_name().to_string();
                if !oid.is_empty() {
                    cb(&oid);
                }
            }
        });

        Self { root, list, on_select }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    pub fn load(&self, commits: Vec<CommitInfo>) {
        // Clear
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        for c in commits {
            let row = gtk::ListBoxRow::new();

            // Layout for each row
            let row_box = gtk::Box::new(Orientation::Vertical, 4);
            row_box.set_margin_top(8);
            row_box.set_margin_bottom(8);
            row_box.set_margin_start(8);
            row_box.set_margin_end(8);

            let top = gtk::Box::new(Orientation::Horizontal, 8);
            let hash = gtk::Label::new(Some(&format!("{}", c.short_id)));
            hash.add_css_class("monospace");
            hash.set_xalign(0.0);

            let summary = gtk::Label::new(Some(&c.summary));
            summary.set_xalign(0.0);
            summary.set_ellipsize(pango::EllipsizeMode::End);
            summary.set_hexpand(true);

            top.append(&hash);
            top.append(&summary);

            let refs = if c.refs.is_empty() { String::new() } else { format!(" • refs: {}", c.refs.join(", ")) };
            let bottom = gtk::Label::new(Some(&format!("{} <{}> • {} • {} parents{}",
                c.author, c.email, c.time, c.parents, refs
            )));
            bottom.add_css_class("dim-label");
            bottom.set_xalign(0.0);

            row_box.append(&top);
            row_box.append(&bottom);

            row.set_child(Some(&row_box));
            row.set_widget_name(&c.oid);

            self.list.append(&row);
        }
    }

    pub fn connect_on_select<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }
}
