use adw::prelude::*;
use gtk::Orientation;
use gtk4 as gtk;
use librefork_core::BranchStatus;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone)]
pub struct SidePanel {
    root: gtk::Box,
    search: gtk::SearchEntry,
    tree: gtk::TreeView,
    store: gtk::TreeStore,
    branches: Rc<RefCell<Vec<BranchStatus>>>,
    remotes: Rc<RefCell<Vec<String>>>,
    starred: Rc<RefCell<HashSet<String>>>,
}

impl SidePanel {
    pub fn new() -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 4);
        root.set_hexpand(false);
        root.set_vexpand(true);

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Rechercher une branche"));
        root.append(&search);

        let store = gtk::TreeStore::new(&[String::static_type(), String::static_type()]);
        let tree = gtk::TreeView::with_model(&store);
        tree.set_headers_visible(false);

        let star_col = gtk::TreeViewColumn::new();
        let star_cell = gtk::CellRendererText::new();
        star_col.pack_start(&star_cell, false);
        star_col.add_attribute(&star_cell, "text", 0);
        tree.append_column(&star_col);

        let name_col = gtk::TreeViewColumn::new();
        let name_cell = gtk::CellRendererText::new();
        name_col.pack_start(&name_cell, true);
        name_col.add_attribute(&name_cell, "text", 1);
        tree.append_column(&name_col);

        root.append(&tree);

        let branches: Rc<RefCell<Vec<BranchStatus>>> = Rc::new(RefCell::new(Vec::new()));
        let remotes: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let starred: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));

        let panel = Self {
            root,
            search: search.clone(),
            tree: tree.clone(),
            store: store.clone(),
            branches: branches.clone(),
            remotes: remotes.clone(),
            starred: starred.clone(),
        };

        search.connect_search_changed({
            let p = panel.clone();
            move |_| {
                p.reload();
            }
        });

        let click = gtk::GestureClick::new();
        let tree_c = tree.clone();
        let store_c = store.clone();
        let starred_c = starred.clone();
        click.connect_pressed(move |g, _, x, y| {
            if g.current_button() == 3 {
                if let Some((Some(path), _col, _x, _y)) = tree_c.path_at_pos(x as i32, y as i32) {
                    if let Some(iter) = store_c.iter(&path) {
                        if store_c.iter_parent(&iter).is_some() {
                            if let Ok(name) = store_c
                                .get_value(&iter, 1)
                                .get::<String>()
                            {
                                let mut st = starred_c.borrow_mut();
                                if st.contains(&name) {
                                    st.remove(&name);
                                    store_c.set(&iter, &[(0u32, &"☆"), (1u32, &name)]);
                                } else {
                                    st.insert(name.clone());
                                    store_c.set(&iter, &[(0u32, &"★"), (1u32, &name)]);
                                }
                            }
                        }
                    }
                }
            }
        });
        tree.add_controller(click);

        panel
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    fn reload(&self) {
        self.store.clear();
        let branches_root = self.store.append(None);
        self.store.set(&branches_root, &[(0, &""), (1, &"Branches")]);
        let remotes_root = self.store.append(None);
        self.store.set(&remotes_root, &[(0, &""), (1, &"Remotes")]);
        let q = self.search.text().to_string().to_lowercase();
        let stars = self.starred.borrow();
        for b in self
            .branches
            .borrow()
            .iter()
            .filter(|b| b.name.to_lowercase().contains(&q))
        {
            let star = if stars.contains(&b.name) { "★" } else { "☆" };
            let label = format!("{} (+{}, -{})", b.name, b.ahead, b.behind);
            let iter = self.store.append(Some(&branches_root));
            self.store.set(&iter, &[(0, &star), (1, &label)]);
        }
        for r in self.remotes.borrow().iter() {
            let iter = self.store.append(Some(&remotes_root));
            self.store.set(&iter, &[(0, &""), (1, r)]);
        }
    }

    pub fn load_branches(&self, branches: &[BranchStatus]) {
        let mut v = self.branches.borrow_mut();
        v.clear();
        v.extend_from_slice(branches);
        drop(v);
        self.reload();
    }

    pub fn load_remotes(&self, remotes: &[String]) {
        let mut v = self.remotes.borrow_mut();
        v.clear();
        v.extend_from_slice(remotes);
        drop(v);
        self.reload();
    }
}

