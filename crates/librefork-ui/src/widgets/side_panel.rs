use crate::starred::StarredItem;
use gtk::gdk;
use gtk::prelude::*;
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
    tags: Rc<RefCell<Vec<String>>>,
    stashes: Rc<RefCell<Vec<String>>>,
    submodules: Rc<RefCell<Vec<String>>>,
    starred: Rc<RefCell<HashSet<StarredItem>>>,
}

impl SidePanel {
    pub fn new(starred: Rc<RefCell<HashSet<StarredItem>>>) -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 4);
        root.set_hexpand(false);
        root.set_vexpand(true);

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Rechercher une branche"));
        root.append(&search);

        let store = gtk::TreeStore::new(&[
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let tree = gtk::TreeView::with_model(&store);
        tree.set_headers_visible(false);
        tree.set_margin_start(4);

        let tree_scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&tree)
            .build();
        tree_scrolled.set_hexpand(true);
        tree_scrolled.set_vexpand(true);

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

        root.append(&tree_scrolled);

        let branches: Rc<RefCell<Vec<BranchStatus>>> = Rc::new(RefCell::new(Vec::new()));
        let remotes: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let tags: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let stashes: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let submodules: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        let panel = Self {
            root,
            search: search.clone(),
            tree: tree.clone(),
            store: store.clone(),
            branches: branches.clone(),
            remotes: remotes.clone(),
            tags: tags.clone(),
            stashes: stashes.clone(),
            submodules: submodules.clone(),
            starred: starred.clone(),
        };

        search.connect_search_changed({
            let p = panel.clone();
            move |_| {
                p.reload();
            }
        });

        let click = gtk::GestureClick::new();
        click.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
        let tree_c = tree.clone();
        let store_c = store.clone();
        let starred_c = starred.clone();
        let panel_c = panel.clone();
        click.connect_pressed(move |_, _, x, y| {
            if let Some((Some(path), _col, _x, _y)) = tree_c.path_at_pos(x as i32, y as i32) {
                if let Some(iter) = store_c.iter(&path) {
                    if store_c.iter_parent(&iter).is_some() {
                        if let (Ok(name), Ok(kind)) = (
                            store_c.get_value(&iter, 1).get::<String>(),
                            store_c.get_value(&iter, 2).get::<String>(),
                        ) {
                            let item = match kind.as_str() {
                                "branch" => StarredItem::Branch(name.clone()),
                                "commit" => StarredItem::Commit(name.clone()),
                                _ => return,
                            };
                            let already = starred_c.borrow().contains(&item);
                            let label = if already {
                                "Retirer des favoris"
                            } else {
                                "Ajouter aux favoris"
                            };
                            let pop = gtk::Popover::new();
                            let bx = gtk::Box::new(Orientation::Vertical, 0);
                            let starred_c2 = starred_c.clone();
                            let pop_c = pop.clone();
                            let panel_c2 = panel_c.clone();
                            let btn = gtk::Button::with_label(label);
                            btn.connect_clicked(move |_| {
                                {
                                    // Ensure the mutable borrow of `starred_c2` is dropped before
                                    // reloading the panel, otherwise `reload` would attempt to
                                    // borrow the same `RefCell` again and panic.
                                    let mut st = starred_c2.borrow_mut();
                                    if already {
                                        st.remove(&item);
                                    } else {
                                        st.insert(item.clone());
                                    }
                                }
                                panel_c2.reload();
                                pop_c.popdown();
                            });
                            bx.append(&btn);
                            pop.set_child(Some(&bx));
                            pop.set_pointing_to(Some(&gdk::Rectangle::new(
                                x as i32, y as i32, 1, 1,
                            )));
                            pop.set_parent(&tree_c);
                            pop.popup();
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

    pub fn reload(&self) {
        if let Some(adj) = self.tree.hadjustment() {
            adj.set_value(0.0);
        }
        self.store.clear();
        let starred_root = self.store.append(None);
        self.store
            .set(&starred_root, &[(0, &""), (1, &"Starred"), (2, &"root")]);
        let branches_root = self.store.append(None);
        self.store
            .set(&branches_root, &[(0, &""), (1, &"Branches"), (2, &"root")]);
        let remotes_root = self.store.append(None);
        self.store
            .set(&remotes_root, &[(0, &""), (1, &"Remotes"), (2, &"root")]);
        let tags_root = self.store.append(None);
        self.store
            .set(&tags_root, &[(0, &""), (1, &"Tags"), (2, &"root")]);
        let stashes_root = self.store.append(None);
        self.store
            .set(&stashes_root, &[(0, &""), (1, &"Stashes"), (2, &"root")]);
        let submodules_root = self.store.append(None);
        self.store.set(
            &submodules_root,
            &[(0, &""), (1, &"Submodules"), (2, &"root")],
        );

        let q = self.search.text().to_string().to_lowercase();
        let stars = self.starred.borrow();
        for item in stars.iter() {
            match item {
                StarredItem::Branch(name) => {
                    if name.to_lowercase().contains(&q) {
                        let iter = self.store.append(Some(&starred_root));
                        self.store
                            .set(&iter, &[(0, &"★"), (1, name), (2, &"branch")]);
                    }
                }
                StarredItem::Commit(oid) => {
                    let short = oid.chars().take(7).collect::<String>();
                    if short.to_lowercase().contains(&q) {
                        let iter = self.store.append(Some(&starred_root));
                        self.store
                            .set(&iter, &[(0, &"★"), (1, &short), (2, &"commit")]);
                    }
                }
            }
        }

        for b in self
            .branches
            .borrow()
            .iter()
            .filter(|b| b.name.to_lowercase().contains(&q))
        {
            let star = if stars.contains(&StarredItem::Branch(b.name.clone())) {
                "★"
            } else {
                "☆"
            };
            let label = format!("{} (+{}, -{})", b.name, b.ahead, b.behind);
            let iter = self.store.append(Some(&branches_root));
            self.store
                .set(&iter, &[(0, &star), (1, &label), (2, &"branch")]);
        }
        for r in self.remotes.borrow().iter() {
            let iter = self.store.append(Some(&remotes_root));
            self.store.set(&iter, &[(0, &""), (1, r), (2, &"remote")]);
        }
        for t in self.tags.borrow().iter() {
            let iter = self.store.append(Some(&tags_root));
            self.store.set(&iter, &[(0, &""), (1, t), (2, &"tag")]);
        }
        for s in self.stashes.borrow().iter() {
            let iter = self.store.append(Some(&stashes_root));
            self.store.set(&iter, &[(0, &""), (1, s), (2, &"stash")]);
        }
        for m in self.submodules.borrow().iter() {
            let iter = self.store.append(Some(&submodules_root));
            self.store
                .set(&iter, &[(0, &""), (1, m), (2, &"submodule")]);
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

    pub fn load_tags(&self, tags: &[String]) {
        let mut v = self.tags.borrow_mut();
        v.clear();
        v.extend_from_slice(tags);
        drop(v);
        self.reload();
    }

    pub fn load_stashes(&self, stashes: &[String]) {
        let mut v = self.stashes.borrow_mut();
        v.clear();
        v.extend_from_slice(stashes);
        drop(v);
        self.reload();
    }

    pub fn load_submodules(&self, subs: &[String]) {
        let mut v = self.submodules.borrow_mut();
        v.clear();
        v.extend_from_slice(subs);
        drop(v);
        self.reload();
    }
}
