use adw::prelude::*;
use gtk::Orientation;
use gtk4 as gtk;
use librefork_core::BranchStatus;

#[derive(Clone)]
pub struct SidePanel {
    root: gtk::Box,
    branches: gtk::ListBox,
    remotes: gtk::ListBox,
}

impl SidePanel {
    pub fn new() -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 4);
        root.set_hexpand(false);
        root.set_vexpand(true);

        let branch_label = gtk::Label::new(Some("Branches"));
        branch_label.add_css_class("title-4");
        branch_label.set_xalign(0.0);
        let branches = gtk::ListBox::new();
        branches.add_css_class("boxed-list");

        let remote_label = gtk::Label::new(Some("Remotes"));
        remote_label.add_css_class("title-4");
        remote_label.set_xalign(0.0);
        let remotes = gtk::ListBox::new();
        remotes.add_css_class("boxed-list");

        root.append(&branch_label);
        root.append(&branches);
        root.append(&remote_label);
        root.append(&remotes);

        Self { root, branches, remotes }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    pub fn load_branches(&self, branches: &[BranchStatus]) {
        while let Some(child) = self.branches.first_child() {
            self.branches.remove(&child);
        }
        for b in branches {
            let label = gtk::Label::new(Some(&format!("{} (+{}, -{})", b.name, b.ahead, b.behind)));
            label.set_xalign(0.0);
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&label));
            self.branches.append(&row);
        }
    }

    pub fn load_remotes(&self, remotes: &[String]) {
        while let Some(child) = self.remotes.first_child() {
            self.remotes.remove(&child);
        }
        for name in remotes {
            let label = gtk::Label::new(Some(name));
            label.set_xalign(0.0);
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&label));
            self.remotes.append(&row);
        }
    }
}
