use adw::prelude::*;
use adw::{Application, ApplicationWindow, StyleManager};
use gtk::Orientation;
use gtk4 as gtk;
use gtk4::{gdk::Display, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::gio; // use the gtk-aligned gio/glib versions
use gtk::glib; // main context + channels for async ops
use librefork_core::RepoHandle;
use std::cell::RefCell;
use std::rc::Rc;

use crate::starred::{StarDb, StarredItem};
use crate::recents::RecentDb;
use glib::prelude::ToVariant;
use crate::widgets::{
    commit_details::CommitDetails, commit_list::{CommitList, CommitContextAction}, side_panel::SidePanel,
};
use std::collections::HashSet;

pub fn build_ui(app: &Application) {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("styles.css"));
    gtk::style_context_add_provider_for_display(
        &Display::default().unwrap(),
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = ApplicationWindow::builder()
        .application(app)
        .title("LibreFork")
        .default_width(1100)
        .default_height(720)
        .build();

    // We'll use a HeaderBar inside an AdwToolbarView (supported for AdwApplicationWindow)
    let header = adw::HeaderBar::new();

    // Window title (shown in the titlebar area, centered)
    window.set_title(Some("LibreFork"));

    // Top menubar (File / View / Repository / Help)
    let root_menu = gio::Menu::new();
    let file_menu = gio::Menu::new();
    file_menu.append(Some("Open Repository…"), Some("app.open"));
    // Recents submenu (populated dynamically)
    let recents_menu = gio::Menu::new();
    file_menu.append_submenu(Some("Recents"), &recents_menu);
    file_menu.append(Some("Quit"), Some("app.quit"));
    root_menu.append_submenu(Some("File"), &file_menu);

    let view_menu = gio::Menu::new();
    // label is updated dynamically later; start with dark mode → offer to switch to light
    view_menu.append(Some("Switch to light theme"), Some("app.toggle-dark"));
    root_menu.append_submenu(Some("View"), &view_menu);

    let repo_menu = gio::Menu::new();
    repo_menu.append(Some("Fetch"), Some("app.fetch"));
    repo_menu.append(Some("Pull"), Some("app.pull"));
    repo_menu.append(Some("Push"), Some("app.push"));
    repo_menu.append(Some("Stash"), Some("app.stash"));
    root_menu.append_submenu(Some("Repository"), &repo_menu);

    let help_menu = gio::Menu::new();
    help_menu.append(Some("About"), Some("app.about"));
    root_menu.append_submenu(Some("Help"), &help_menu);

    let menubar = gtk::PopoverMenuBar::from_model(Some(&root_menu));
    menubar.add_css_class("compact-menubar");

    // We rely on AdwApplicationWindow's built-in titlebar with system buttons.

    // Second row: icon toolbar (Fetch / Pull / Push / Stash) + branch + theme
    let toolbar = gtk::Box::new(Orientation::Horizontal, 8);
    toolbar.add_css_class("toolbar");
    toolbar.add_css_class("compact-toolbar");

    fn tool_button(icon: &str, label: &str) -> gtk::Button {
        let btn = gtk::Button::new();
        btn.add_css_class("flat");
        let v = gtk::Box::new(Orientation::Vertical, 2);
        v.set_halign(gtk::Align::Center);
        v.set_valign(gtk::Align::Center);
        let img = gtk::Image::from_icon_name(icon);
        let lbl = gtk::Label::new(Some(label));
        lbl.add_css_class("dim-label");
        v.append(&img);
        v.append(&lbl);
        btn.set_child(Some(&v));
        btn
    }

    let fetch_button = tool_button("emblem-synchronizing-symbolic", "Fetch");
    let pull_button = tool_button("go-down-symbolic", "Pull");
    let push_button = tool_button("go-up-symbolic", "Push");
    let stash_button = tool_button("document-save-symbolic", "Stash");
    let refresh_button = tool_button("view-refresh-symbolic", "Refresh");

    toolbar.append(&fetch_button);
    toolbar.append(&pull_button);
    toolbar.append(&push_button);
    toolbar.append(&stash_button);
    toolbar.append(&refresh_button);

    let toolbar_spacer = gtk::Box::new(Orientation::Horizontal, 0);
    toolbar_spacer.set_hexpand(true);
    toolbar.append(&toolbar_spacer);

    // let settings = gtk::Settings::default().expect("Could not get default settings");
    let style_manager = StyleManager::default();
    // Force démarrage en dark mode
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);

    // Main layout
    let paned = gtk::Paned::builder()
        .orientation(Orientation::Vertical)
        .start_child(&gtk::Label::new(None))
        .end_child(&gtk::Label::new(None))
        .wide_handle(true)
        .build();

    let commit_scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    let details_scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let star_db = Rc::new(StarDb::new().expect("failed to init star db"));
    let recent_db = Rc::new(RecentDb::new().expect("failed to init recents db"));
    let starred: Rc<RefCell<HashSet<StarredItem>>> = Rc::new(RefCell::new(HashSet::new()));
    let prev_stars: Rc<RefCell<HashSet<StarredItem>>> = Rc::new(RefCell::new(HashSet::new()));
    let commit_list = CommitList::new(starred.clone());
    let details = CommitDetails::new();
    let side_panel = SidePanel::new(starred.clone());

    commit_scrolled.set_child(Some(commit_list.widget()));
    details_scrolled.set_child(Some(details.widget()));

    let load_more_button = gtk::Button::with_label("Charger plus");
    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Rechercher"));
    let top_box = gtk::Box::new(Orientation::Vertical, 0);
    top_box.append(&search_entry);
    top_box.append(&commit_scrolled);
    top_box.append(&load_more_button);

    paned.set_start_child(Some(&top_box));
    paned.set_end_child(Some(&details_scrolled));
    paned.set_position(300);

    let outer = gtk::Paned::builder()
        .orientation(Orientation::Horizontal)
        .start_child(side_panel.widget())
        .end_child(&paned)
        .wide_handle(true)
        .build();
    outer.set_position(200);

    let content = gtk::Box::new(Orientation::Vertical, 0);
    content.append(&menubar);
    content.append(&toolbar);
    content.append(&outer);

    // Status bar (bottom): left → branch + sync counts (clickable); right → activity spinner + message
    let status_bar = gtk::Box::new(Orientation::Horizontal, 8);
    status_bar.add_css_class("toolbar");
    let status_left_box = gtk::Box::new(Orientation::Horizontal, 8);
    status_left_box.set_halign(gtk::Align::Start);
    let status_branch_label = gtk::Label::new(None);
    status_branch_label.add_css_class("dim-label");
    let status_sync_button = gtk::Button::with_label("🗘 0↓ 0↑");
    status_sync_button.add_css_class("flat");
    status_sync_button.add_css_class("dim-label");
    status_sync_button.set_sensitive(false);
    status_sync_button.set_tooltip_text(Some("No repository open"));
    status_left_box.append(&status_branch_label);
    status_left_box.append(&status_sync_button);
    let status_spacer = gtk::Box::new(Orientation::Horizontal, 0);
    status_spacer.set_hexpand(true);
    let activity_spinner = gtk::Spinner::new();
    activity_spinner.set_spinning(false);
    let activity_label = gtk::Label::new(Some("Prêt"));
    activity_label.add_css_class("dim-label");
    status_bar.append(&status_left_box);
    status_bar.append(&status_spacer);
    status_bar.append(&activity_spinner);
    status_bar.append(&activity_label);
    content.append(&status_bar);

    // Wrap content into a ToolbarView and add the header as top bar
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&content));

    window.set_content(Some(&toolbar_view));

    {
        let commit_list_c = commit_list.clone();
        search_entry.connect_search_changed(move |entry| {
            commit_list_c.filter(&entry.text());
        });
    }

    // State
    #[derive(Default, Clone)]
    struct State {
        // Current selected repo path
        repo_path: Option<String>,
        // Per-repo pagination state
        loaded_by_repo: std::collections::HashMap<String, usize>,
        // Open repositories list (for tabs)
        repos: Vec<String>,
        // Current selected tab index (if any)
        current_idx: Option<usize>,
    }
    let state = Rc::new(RefCell::new(State::default()));
    const PAGE_SIZE: usize = 100;

    let commit_list_c = commit_list.clone();
    let side_c = side_panel.clone();
    let state_c = state.clone();
    let starred_c = starred.clone();
    let prev_c = prev_stars.clone();
    let star_db_c = star_db.clone();
    let search_entry_c = search_entry.clone();
    let star_cb = move || {
        if let Some(repo) = state_c.borrow().repo_path.clone() {
            let new_set = starred_c.borrow();
            let prev_set = prev_c.borrow();
            for item in new_set.difference(&prev_set) {
                let _ = star_db_c.add(&repo, item);
            }
            for item in prev_set.difference(&new_set) {
                let _ = star_db_c.remove(&repo, item);
            }
        }
        *prev_c.borrow_mut() = starred_c.borrow().clone();
        side_c.reload();
        commit_list_c.filter(&search_entry_c.text());
    };
    commit_list.on_star_changed(star_cb.clone());
    side_panel.on_star_changed(star_cb);

    fn load_repo(
        path: &str,
        state: &Rc<RefCell<State>>,
        commit_list: &CommitList,
        details: &CommitDetails,
        side: &SidePanel,
        window: &ApplicationWindow,
        load_more: &gtk::Button,
        search_entry: &gtk::SearchEntry,
        starred: &Rc<RefCell<HashSet<StarredItem>>>,
        prev_stars: &Rc<RefCell<HashSet<StarredItem>>>,
        star_db: &StarDb,
        status_branch_label: &gtk::Label,
        status_sync_button: &gtk::Button,
    ) {
        match RepoHandle::open(path) {
            Ok(repo) => {
                if let Ok(stars) = star_db.load(path) {
                    let mut st = starred.borrow_mut();
                    st.clear();
                    st.extend(stars);
                    *prev_stars.borrow_mut() = st.clone();
                }

                if let Ok(head) = repo.head() {
                    if let Some(name) = head {
                        window.set_title(Some(&format!("LibreFork - {}", name)));
                    }
                }

                // Branches are shown in the left panel; no combo selector here anymore

                if let Ok(statuses) = repo.list_branches_with_upstream() {
                    side.load_branches(&statuses);
                }
                if let Ok(remotes) = repo.list_remotes() {
                    side.load_remotes(&remotes);
                }
                if let Ok(tags) = repo.list_tags() {
                    side.load_tags(&tags);
                }
                if let Ok(stashes) = repo.list_stashes() {
                    side.load_stashes(&stashes);
                }
                if let Ok(subs) = repo.list_submodules() {
                    side.load_submodules(&subs);
                }

                let mut st = state.borrow_mut();
                st.repo_path = Some(path.to_string());
                // ensure repo in tabs list
                if !st.repos.iter().any(|p| p == path) {
                    st.repos.push(path.to_string());
                }
                // update selected idx
                st.current_idx = st.repos.iter().position(|p| p == path);
                st.loaded_by_repo.insert(path.to_string(), 0);

                if let Ok(commits) = repo.list_commits_paginated(0, PAGE_SIZE) {
                    commit_list.load(commits.clone());
                    details.clear();
                    st.loaded_by_repo.insert(path.to_string(), commits.len());
                    load_more.set_sensitive(commits.len() == PAGE_SIZE);
                    search_entry.set_text("");
                    commit_list.filter("");
                }

                // Update status bar: branch label + clickable sync counts
                let mut status_text = String::new();
                // sync segment text will be set on the button directly
                if let Ok(head) = repo.head() {
                    if let Some(branch) = head {
                        status_text = format!("⎇ {} —", branch);
                        if let Ok(statuses) = repo.list_branches_with_upstream() {
                            if let Some(s) = statuses.iter().find(|s| s.name == branch) {
                                status_sync_button.set_label(&format!("🗘 {}↓ {}↑", s.behind, s.ahead));
                                status_sync_button.set_sensitive(true);
                                status_sync_button.set_tooltip_text(Some(&format!(
                                    "Pull {} et push {} vers {}",
                                    s.behind, s.ahead, branch
                                )));
                            }
                        }
                    }
                }
                if status_text.is_empty() {
                    status_text = "Aucun dépôt ouvert".to_string();
                    status_sync_button.set_label("🗘 0↓ 0↑");
                    status_sync_button.set_sensitive(false);
                    status_sync_button.set_tooltip_text(Some("Aucun dépôt ouvert"));
                }
                status_branch_label.set_text(&status_text);
            }
            Err(err) => eprintln!("Erreur d'ouverture du dépôt: {err}"),
        }
    }

    // Simple tab row with a "+" opener; visible only when >= 2 repos
    let tabs_row = gtk::Box::new(Orientation::Horizontal, 6);
    tabs_row.add_css_class("toolbar");
    tabs_row.add_css_class("tabbar");
    let plus_button = gtk::Button::from_icon_name("list-add-symbolic");
    plus_button.add_css_class("flat");
    plus_button.set_tooltip_text(Some("Ouvrir un dépôt dans un nouvel onglet"));
    // Insert tabs row after the toolbar in the vertical content box
    content.insert_child_after(&tabs_row, Some(&toolbar));
    // Helper to rebuild tabs based on state
    let rebuild_tabs = {
        let tabs_row = tabs_row.clone();
        let plus_button = plus_button.clone();
        let state = state.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let side_c = side_panel.clone();
        let window_c = window.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();
        let starred_c = starred.clone();
        let prev_c = prev_stars.clone();
        let star_db_c = star_db.clone();
        let status_branch_label_c = status_branch_label.clone();
        let status_sync_button_c_global = status_sync_button.clone();
        move || {
            // Clear row
            while let Some(child) = tabs_row.first_child() { tabs_row.remove(&child); }

            let (repos, current_idx) = {
                let st = state.borrow();
                (st.repos.clone(), st.current_idx)
            };
            for (i, p) in repos.iter().enumerate() {
                let label = std::path::Path::new(p)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(p)
                    .replace('_', "__");
                let tab_box = gtk::Box::new(Orientation::Horizontal, 2);
                tab_box.add_css_class("tab-item");
                let b = gtk::ToggleButton::with_label(&label);
                b.add_css_class("flat");
                b.add_css_class("tab-button");
                if Some(i) == current_idx {
                    b.set_active(true);
                    b.add_css_class("tab-active");
                }
                // Tooltip: chemin complet
                b.set_tooltip_text(Some(p));
                let close_btn = gtk::Button::from_icon_name("window-close-symbolic");
                close_btn.add_css_class("flat");
                close_btn.add_css_class("tab-close");
                close_btn.set_tooltip_text(Some("Fermer l'onglet"));

                let p2 = p.clone();
                let state_c = state.clone();
                let commit_list_c2 = commit_list_c.clone();
                let details_c2 = details_c.clone();
                let side_c2 = side_c.clone();
                let window_c2 = window_c.clone();
                let load_more_c2 = load_more_c.clone();
                let search_entry_c2 = search_entry_c.clone();
                let starred_c2 = starred_c.clone();
                let prev_c2 = prev_c.clone();
                let star_db_c2 = star_db_c.clone();
                let status_branch_label_c2 = status_branch_label_c.clone();
                let status_sync_button_c = status_sync_button_c_global.clone();
                {
                let tabs_row_c = tabs_row.clone();
                let b_c = b.clone();
                b.connect_clicked(move |_| {
                    {
                        let mut st = state_c.borrow_mut();
                        st.repo_path = Some(p2.clone());
                        st.current_idx = st.repos.iter().position(|x| x == &p2);
                    }
                    // Update active tab visual state
                    let mut child = tabs_row_c.first_child();
                    while let Some(c) = child {
                        let next = c.next_sibling();
                        if let Ok(container) = c.downcast::<gtk::Box>() {
                            if let Some(first) = container.first_child() {
                                if let Ok(tb) = first.downcast::<gtk::ToggleButton>() {
                                    tb.remove_css_class("tab-active");
                                }
                            }
                        }
                        child = next;
                    }
                    // Set active on this one
                    b_c.add_css_class("tab-active");
                    load_repo(
                        &p2,
                        &state_c,
                        &commit_list_c2,
                        &details_c2,
                        &side_c2,
                        &window_c2,
                        &load_more_c2,
                        &search_entry_c2,
                        &starred_c2,
                        &prev_c2,
                        &star_db_c2,
                        &status_branch_label_c2,
                        &status_sync_button_c,
                    );
                });
                }

                // Close behavior
                {
                    let state_c = state.clone();
                    let p_close = p.clone();
                    let commit_list_c2 = commit_list_c.clone();
                    let details_c2 = details_c.clone();
                    let side_c2 = side_c.clone();
                    let window_c2 = window_c.clone();
                    let load_more_c2 = load_more_c.clone();
                    let search_entry_c2 = search_entry_c.clone();
                    let starred_c2 = starred_c.clone();
                    let prev_c2 = prev_c.clone();
                    let star_db_c2 = star_db_c.clone();
                    let status_branch_label_c2 = status_branch_label_c.clone();
                    let tabs_row_c = tabs_row.clone();
                    let status_sync_button_c2 = status_sync_button_c_global.clone();
                    let tab_box_c = tab_box.clone();
                    close_btn.connect_clicked(move |_| {
                        let (maybe_new_path, need_reload) = {
                            let mut st = state_c.borrow_mut();
                            if let Some(idx) = st.repos.iter().position(|x| x == &p_close) {
                                st.repos.remove(idx);
                                st.loaded_by_repo.remove(&p_close);
                                if let Some(cur) = st.current_idx {
                                    if cur == idx {
                                        if st.repos.is_empty() {
                                            st.current_idx = None;
                                            st.repo_path = None;
                                            (None, true)
                                        } else {
                                            let new_idx = if idx >= st.repos.len() { st.repos.len() - 1 } else { idx };
                                            st.current_idx = Some(new_idx);
                                            let new_path = st.repos[new_idx].clone();
                                            st.repo_path = Some(new_path.clone());
                                            (Some(new_path), true)
                                        }
                                    } else {
                                        if let Some(ci) = st.current_idx { st.current_idx = Some(ci.min(st.repos.len().saturating_sub(1))); }
                                        (st.repo_path.clone(), false)
                                    }
                                } else {
                                    (None, false)
                                }
                            } else {
                                (st.repo_path.clone(), false)
                            }
                        };
                        // Remove this tab UI
                        tabs_row_c.remove(&tab_box_c);
                        // Tabbar stays visible even with < 2 repos
                        if need_reload {
                            if let Some(path) = maybe_new_path {
                                // Update active CSS on remaining tabs
                                let target_label = std::path::Path::new(&path)
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(&path)
                                    .replace('_', "__");
                                let mut child = tabs_row_c.first_child();
                                while let Some(c) = child {
                                    let next = c.next_sibling();
                                    if let Ok(container) = c.downcast::<gtk::Box>() {
                                        if let Some(first) = container.first_child() {
                                            if let Ok(tb) = first.downcast::<gtk::ToggleButton>() {
                                                tb.remove_css_class("tab-active");
                                                if let Some(lbl) = tb.label() {
                                                    if lbl.as_str() == target_label {
                                                        tb.add_css_class("tab-active");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    child = next;
                                }
                                load_repo(
                                    &path,
                                    &state_c,
                                    &commit_list_c2,
                                    &details_c2,
                                    &side_c2,
                                    &window_c2,
                                    &load_more_c2,
                                    &search_entry_c2,
                                    &starred_c2,
                                    &prev_c2,
                                    &star_db_c2,
                                    &status_branch_label_c2,
                                    &status_sync_button_c2,
                                );
                            } else {
                                commit_list_c2.load(Vec::new());
                                details_c2.clear();
                                side_c2.reload();
                                status_branch_label_c2.set_text("Aucun dépôt ouvert");
                                status_sync_button_c2.set_label("🗘 0↓ 0↑");
                                status_sync_button_c2.set_sensitive(false);
                                status_sync_button_c2.set_tooltip_text(Some("No repository open"));
                            }
                        }
                    });
                }

                tab_box.append(&b);
                tab_box.append(&close_btn);
                tabs_row.append(&tab_box);
            }
            // Always show the plus button and the tabbar
            tabs_row.append(&plus_button);
            tabs_row.set_visible(true);
        }
    };

    // Helper to refresh the Recents submenu from the DB
    let refresh_recents_menu = {
        let recents_menu = recents_menu.clone();
        let recent_db = recent_db.clone();
        move || {
            while recents_menu.n_items() > 0 { recents_menu.remove(0); }
            if let Ok(paths) = recent_db.list(10) {
                for p in paths.iter() {
                    let raw_label = std::path::Path::new(p)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(p)
                        .to_string();
                    // Escape underscores so GTK doesn't treat them as mnemonics
                    let label = raw_label.replace('_', "__");
                    let item = gio::MenuItem::new(Some(&label), Some("app.open-recent"));
                    item.set_attribute_value("target", Some(&p.to_variant()));
                    item.set_attribute_value("tooltip", Some(&p.to_variant()));
                    recents_menu.append_item(&item);
                }
                if !paths.is_empty() {
                    let item = gio::MenuItem::new(Some("Clear List"), Some("app.clear-recents"));
                    recents_menu.append_item(&item);
                }
            }
        }
    };

    // Interactions
    {
        let state = state.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let side_c = side_panel.clone();
        let window_c = window.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();
        let starred_c = starred.clone();
        let prev_c = prev_stars.clone();
        let star_db_c = star_db.clone();
        let status_branch_label_c = status_branch_label.clone();
        let status_sync_button_c = status_sync_button.clone();
        refresh_button.connect_clicked(move |_| {
            let path_opt = { state.borrow().repo_path.clone() };
            if let Some(path) = path_opt {
                load_repo(
                    &path,
                    &state,
                    &commit_list_c,
                    &details_c,
                    &side_c,
                    &window_c,
                    &load_more_c,
                    &search_entry_c,
                    &starred_c,
                    &prev_c,
                    &star_db_c,
                    &status_branch_label_c,
                    &status_sync_button_c,
                );
            }
        });
    }

    // Actions: repository operations (also wired to buttons) with async + statusbar update
    // Helper to run a repo op off the main thread and refresh UI upon completion
    let run_repo_op = {
        let state = state.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let side_c = side_panel.clone();
        let window_c = window.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();
        let starred_c = starred.clone();
        let prev_c = prev_stars.clone();
        let star_db_c = star_db.clone();
        let status_branch_label_c = status_branch_label.clone();
        let status_sync_button_c = status_sync_button.clone();
        let spinner = activity_spinner.clone();
        let label = activity_label.clone();
        move |op_name: &'static str, f: Box<dyn FnOnce(&RepoHandle) -> anyhow::Result<()> + Send>| {
            let path_opt = { state.borrow().repo_path.clone() };
            if path_opt.is_none() { return; }
            let path = path_opt.unwrap();
            spinner.set_spinning(true);
            label.set_text(&format!("{} en cours…", op_name));

            let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
            std::thread::spawn(move || {
                let res = match RepoHandle::open(&path) {
                    Ok(repo) => f(&repo).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = tx.send(res);
            });

            let state = state.clone();
            let commit_list_c = commit_list_c.clone();
            let details_c = details_c.clone();
            let side_c = side_c.clone();
            let window_c = window_c.clone();
            let load_more_c = load_more_c.clone();
            let search_entry_c = search_entry_c.clone();
            let starred_c = starred_c.clone();
            let prev_c = prev_c.clone();
            let star_db_c = star_db_c.clone();
            let status_branch_label_c = status_branch_label_c.clone();
            let spinner_c = spinner.clone();
            let label_c = label.clone();
            let status_sync_button_c2 = status_sync_button_c.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                use std::sync::mpsc::TryRecvError;
                match rx.try_recv() {
                    Ok(result) => {
                        spinner_c.set_spinning(false);
                        match result {
                            Ok(_) => label_c.set_text("Prêt"),
                            Err(err) => label_c.set_text(&format!("{}: {}", op_name, err)),
                        }
                        let path_opt = { state.borrow().repo_path.clone() };
                        if let Some(path) = path_opt {
                            load_repo(
                                &path,
                                &state,
                                &commit_list_c,
                                &details_c,
                                &side_c,
                                &window_c,
                                &load_more_c,
                                &search_entry_c,
                                &starred_c,
                                &prev_c,
                                &star_db_c,
                                &status_branch_label_c,
                                &status_sync_button_c2,
                            );
                        }
                        glib::ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        spinner_c.set_spinning(false);
                        label_c.set_text(&format!("{}: channel fermé", op_name));
                        glib::ControlFlow::Break
                    }
                }
            });
        }
    };

    // Variant that accepts a completion callback to allow chaining operations
    let run_repo_op_with_cb: std::rc::Rc<dyn Fn(
        &'static str,
        Box<dyn FnOnce(&RepoHandle) -> anyhow::Result<()> + Send>,
        Option<Box<dyn FnOnce()>>,
    )> = {
        let state = state.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let side_c = side_panel.clone();
        let window_c = window.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();
        let starred_c = starred.clone();
        let prev_c = prev_stars.clone();
        let star_db_c = star_db.clone();
        let status_branch_label_c = status_branch_label.clone();
        let status_sync_button_c = status_sync_button.clone();
        let spinner = activity_spinner.clone();
        let label = activity_label.clone();
        std::rc::Rc::new(move |op_name, f, next| {
            let path_opt = { state.borrow().repo_path.clone() };
            if path_opt.is_none() { return; }
            let path = path_opt.unwrap();
            spinner.set_spinning(true);
            label.set_text(&format!("{} en cours…", op_name));

            let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
            std::thread::spawn(move || {
                let res = match RepoHandle::open(&path) {
                    Ok(repo) => f(&repo).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let _ = tx.send(res);
            });

            let state = state.clone();
            let commit_list_c = commit_list_c.clone();
            let details_c = details_c.clone();
            let side_c = side_c.clone();
            let window_c = window_c.clone();
            let load_more_c = load_more_c.clone();
            let search_entry_c = search_entry_c.clone();
            let starred_c = starred_c.clone();
            let prev_c = prev_c.clone();
            let star_db_c = star_db_c.clone();
            let status_branch_label_c = status_branch_label_c.clone();
            let spinner_c = spinner.clone();
            let label_c = label.clone();
            let status_sync_button_c2 = status_sync_button_c.clone();
            let next_cb: std::rc::Rc<std::cell::RefCell<Option<Box<dyn FnOnce()>>>> = std::rc::Rc::new(std::cell::RefCell::new(next));
            let next_cb_c = next_cb.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                use std::sync::mpsc::TryRecvError;
                match rx.try_recv() {
                    Ok(result) => {
                        spinner_c.set_spinning(false);
                        match result {
                            Ok(_) => label_c.set_text("Prêt"),
                            Err(err) => label_c.set_text(&format!("{}: {}", op_name, err)),
                        }
                        let path_opt = { state.borrow().repo_path.clone() };
                        if let Some(path) = path_opt {
                            load_repo(
                                &path,
                                &state,
                                &commit_list_c,
                                &details_c,
                                &side_c,
                                &window_c,
                                &load_more_c,
                                &search_entry_c,
                                &starred_c,
                                &prev_c,
                                &star_db_c,
                                &status_branch_label_c,
                                &status_sync_button_c2,
                            );
                        }
                        if let Some(cb) = next_cb_c.borrow_mut().take() {
                            cb();
                        }
                        glib::ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(TryRecvError::Disconnected) => {
                        spinner_c.set_spinning(false);
                        label_c.set_text(&format!("{}: channel fermé", op_name));
                        glib::ControlFlow::Break
                    }
                }
            });
        })
    };

    {
        let run_repo_op_c = run_repo_op.clone();
        fetch_button.connect_clicked(move |_| {
            run_repo_op_c("Fetch", Box::new(|repo| repo.fetch().map_err(|e| e.into())));
        });
    }
    {
        let run_repo_op_c = run_repo_op.clone();
        pull_button.connect_clicked(move |_| {
            run_repo_op_c("Pull", Box::new(|repo| repo.pull().map_err(|e| e.into())));
        });
    }
    {
        // Push operation not implemented in core yet; placeholder uses pull to avoid UI breakage
        let run_repo_op_c = run_repo_op.clone();
        push_button.connect_clicked(move |_| {
            run_repo_op_c("Push", Box::new(|repo| repo.pull().map_err(|e| e.into())));
        });
    }
    {
        let run_repo_op_c = run_repo_op.clone();
        stash_button.connect_clicked(move |_| {
            run_repo_op_c("Stash", Box::new(|repo| {
                let mut r = RepoHandle::open(&repo.path)?; // need mutable for stash
                r.stash("WIP").map_err(|e| e.into())
            }));
        });
    }

    // Status bar sync segment: chain Fetch → Pull → Push when clicked
    {
        let run_seq = run_repo_op_with_cb.clone();
        status_sync_button.connect_clicked(move |_| {
            let run_seq_pull = run_seq.clone();
            let run_seq_push = run_seq.clone();
            run_seq(
                "Fetch",
                Box::new(|repo| repo.fetch().map_err(|e| e.into())),
                Some(Box::new(move || {
                    run_seq_pull(
                        "Pull",
                        Box::new(|repo| repo.pull().map_err(|e| e.into())),
                        Some(Box::new(move || {
                            // Push placeholder uses pull until implemented in core
                            run_seq_push(
                                "Push",
                                Box::new(|repo| repo.pull().map_err(|e| e.into())),
                                None,
                            );
                        })),
                    );
                })),
            );
        });
    }

    // Side panel double-click: toggle parents or checkout branches/tags
    {
        let run_repo_op_c = run_repo_op.clone();
        side_panel.on_activate(move |kind, name| {
            match kind.as_str() {
                "branch" => {
                    let n = name.clone();
                    run_repo_op_c("Checkout", Box::new(move |repo| repo.checkout_branch(&n).map_err(|e| e.into())));
                }
                "tag" => {
                    let n = name.clone();
                    run_repo_op_c("Checkout", Box::new(move |repo| repo.checkout_tag(&n).map_err(|e| e.into())));
                }
                _ => {}
            }
        });
    }

    {
        let state_for_dialog_open = state.clone();
        let window_for_open = window.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let side_c_cloned = side_panel.clone(); // Clone du Rc ici
        let window = window.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();
        let starred_c = starred.clone();
        let prev_c = prev_stars.clone();
        let star_db_c = star_db.clone();

        // Holder pour garder le dialog vivant jusqu'à la réponse
        let dialog_holder: Rc<RefCell<Option<gtk::FileChooserNative>>> =
            Rc::new(RefCell::new(None));

        // Shared handler for opening a repository (used by menu action too)
        let status_branch_label_for_open = status_branch_label.clone();
        let status_sync_button_for_open = status_sync_button.clone();
        let recent_db_for_open = recent_db.clone();
        let refresh_recents_menu_for_open = refresh_recents_menu.clone();
        let run_repo_op_for_open = run_repo_op.clone();
        let rebuild_tabs_for_open = rebuild_tabs.clone();
        let open_repo_handler_with_policy: Rc<dyn Fn(bool)> = Rc::new(move |force_add: bool| {
            let dialog = gtk::FileChooserNative::builder()
                .title("Ouvrir un dépôt Git")
                .action(gtk::FileChooserAction::SelectFolder)
                .transient_for(&window_for_open) // parent
                .modal(true)
                .build();

            // conservez une ref forte
            *dialog_holder.borrow_mut() = Some(dialog.clone());

            let status_lbl_capture = status_branch_label_for_open.clone();
            let status_sync_btn_capture = status_sync_button_for_open.clone();
            let recent_db_for_open_c = recent_db_for_open.clone();
            let refresh_recents_menu_for_open_c = refresh_recents_menu_for_open.clone();
            let run_repo_op_for_open_c = run_repo_op_for_open.clone();
            dialog.connect_response({
        let state_for_dialog_cb = state_for_dialog_open.clone();
        let commit_list_c = commit_list_c.clone();
        let details_c = details_c.clone();
        let side_c_cloned = side_c_cloned.clone(); // Cloner ici
                let window_c2 = window_for_open.clone();
                let load_more_c2 = load_more_c.clone();
                let search_entry_c2 = search_entry_c.clone();
                let starred_c2 = starred_c.clone();
                let prev_c2 = prev_c.clone();
                let star_db_c2 = star_db_c.clone();
                let holder = dialog_holder.clone();
                let status_lbl_c2 = status_lbl_capture.clone();
                let status_sync_btn_c2 = status_sync_btn_capture.clone();
                let rebuild_tabs_c = rebuild_tabs_for_open.clone();

                move |dlg, resp| {
                    holder.borrow_mut().take();

                    if resp == gtk::ResponseType::Accept {
                        if let Some(file) = dlg.file() {
                            if let Some(path) = file.path() {
                                let chosen = path.to_string_lossy().to_string();
                                let len_before = { state_for_dialog_cb.borrow().repos.len() };
                                if force_add {
                                    {
                                        let mut st = state_for_dialog_cb.borrow_mut();
                                        if !st.repos.iter().any(|p| p == &chosen) {
                                            st.repos.push(chosen.clone());
                                        }
                                        st.current_idx = st.repos.iter().position(|p| p == &chosen);
                                    }
                                    rebuild_tabs_c();
                load_repo(
                    &chosen,
                    &state_for_dialog_cb,
                    &commit_list_c,
                    &details_c,
                    &side_c_cloned,
                    &window_c2,
                    &load_more_c2,
                    &search_entry_c2,
                    &starred_c2,
                    &prev_c2,
                    &star_db_c2,
                    &status_lbl_c2,
                    &status_sync_btn_c2,
                );
                                } else if len_before == 1 {
                                    // Ask user: replace or add
                                    let dlg = adw::MessageDialog::new(
                                        Some(&window_c2),
                                        Some("Ouvrir le dépôt"),
                                        Some("Voulez-vous remplacer le dépôt actuel ou l'ajouter dans un nouvel onglet ?"),
                                    );
                                    dlg.add_response("replace", "Remplacer");
                                    dlg.add_response("add", "Ajouter");
                                    dlg.set_default_response(Some("add"));
                                    let state_c3 = state_for_dialog_cb.clone();
                                    let commit_list_c3 = commit_list_c.clone();
                                    let details_c3 = details_c.clone();
                                    let side_c3 = side_c_cloned.clone();
                                    let window_c3 = window_c2.clone();
                                    let load_more_c3 = load_more_c2.clone();
                                    let search_entry_c3 = search_entry_c2.clone();
                                    let starred_c3 = starred_c2.clone();
                                    let prev_c3 = prev_c2.clone();
                                    let star_db_c3 = star_db_c2.clone();
                                    let status_lbl_c3 = status_lbl_c2.clone();
                                    let status_sync_btn_c3 = status_sync_btn_c2.clone();
                                    let rebuild_tabs_c3 = rebuild_tabs_c.clone();
                                    let recent_db_c3 = recent_db_for_open_c.clone();
                                    let refresh_recents_menu_c3 = refresh_recents_menu_for_open_c.clone();
                                    let run_repo_op_c3 = run_repo_op_for_open_c.clone();
                                    let chosen_c = chosen.clone();
                                    dlg.connect_response(None, move |d: &adw::MessageDialog, resp: &str| {
                                        d.hide();
        
                                        if resp == "replace" {
                                            {
                                                let mut st = state_c3.borrow_mut();
                                                st.repos.clear();
                                                st.repos.push(chosen_c.clone());
                                                st.current_idx = Some(0);
                                            }
                                        } else {
                                            let mut st = state_c3.borrow_mut();
                                            if !st.repos.iter().any(|p| p == &chosen_c) {
                                                st.repos.push(chosen_c.clone());
                                            }
                                            st.current_idx = st.repos.iter().position(|p| p == &chosen_c);
                                        }
                                        rebuild_tabs_c3();
                                        load_repo(
                                            &chosen_c,
                                            &state_c3,
                                            &commit_list_c3,
                                            &details_c3,
                                            &side_c3,
                                            &window_c3,
                                            &load_more_c3,
                                            &search_entry_c3,
                                            &starred_c3,
                                            &prev_c3,
                                            &star_db_c3,
                                            &status_lbl_c3,
                                            &status_sync_btn_c3,
                                        );
                                        // Update recents and auto-fetch
                                        let _ = recent_db_c3.touch(&chosen_c);
                                        refresh_recents_menu_c3();
                                        run_repo_op_c3("Fetch", Box::new(|repo| repo.fetch().map_err(|e| e.into())));
                                    });
                                    dlg.present();
                                } else {
                                    // 0 or >=2 open repos: add/select
                                    {
                                        let mut st = state_for_dialog_cb.borrow_mut();
                                        if !st.repos.iter().any(|p| p == &chosen) {
                                            st.repos.push(chosen.clone());
                                        }
                                        st.current_idx = st.repos.iter().position(|p| p == &chosen);
                                    }
                        rebuild_tabs_c();
                        load_repo(
                            &chosen,
                            &state_for_dialog_cb,
                            &commit_list_c,
                            &details_c,
                            &side_c_cloned,
                            &window_c2,
                            &load_more_c2,
                            &search_entry_c2,
                            &starred_c2,
                            &prev_c2,
                            &star_db_c2,
                            &status_lbl_c2,
                            &status_sync_btn_c2,
                        );
                                }
                                // Update recents list and menu (for force_add or 0/>=2 branch; for ask branch handled inside)
                                let _ = recent_db_for_open_c.touch(path.to_string_lossy().as_ref());
                                refresh_recents_menu_for_open_c();
                                // Auto-fetch after open
                                run_repo_op_for_open_c("Fetch", Box::new(|repo| repo.fetch().map_err(|e| e.into())));
                            }
                        }
                    }
                }
            });

            dialog.show();
        });

        // Keep a hidden button for code reuse if needed
        let open_button = gtk::Button::with_label("Ouvrir un dépôt…");
        let handler_btn = open_repo_handler_with_policy.clone();
        open_button.connect_clicked(move |_| (handler_btn)(false));
        // Open via + tab always adds
        let handler_plus = open_repo_handler_with_policy.clone();
        plus_button.connect_clicked(move |_| (handler_plus)(true));

        // Application actions
        let app_weak = app.downgrade();
        let act_open = gio::SimpleAction::new("open", None);
        let handler_action = open_repo_handler_with_policy.clone();
        act_open.connect_activate(move |_, _| (handler_action)(false));
        app.add_action(&act_open);

        // Open a recent repo via action parameter (String path)
        let act_open_recent = gio::SimpleAction::new("open-recent", Some(&String::static_variant_type()));
        let state_for_recent = state.clone();
        let commit_list_rc = commit_list.clone();
        let details_rc = details.clone();
        let side_rc = side_panel.clone();
        let window_rc = window.clone();
        let load_more_rc = load_more_button.clone();
        let search_entry_rc = search_entry.clone();
        let starred_rc = starred.clone();
        let prev_rc = prev_stars.clone();
        let star_db_rc = star_db.clone();
        let status_lbl_rc = status_branch_label.clone();
        let status_sync_button_rc = status_sync_button.clone();
        let recent_db_rc = recent_db.clone();
        let refresh_recents_menu_rc = refresh_recents_menu.clone();
        let run_repo_op_rc = run_repo_op.clone();
        let rebuild_tabs_rc = rebuild_tabs.clone();
        act_open_recent.connect_activate(move |_, param| {
            if let Some(v) = param {
                if let Some(s) = v.str() {
                    let p = s.to_string();
                    let count = { state_for_recent.borrow().repos.len() };
                    if count == 1 {
                        let dlg = adw::MessageDialog::new(
                            Some(&window_rc),
                            Some("Ouvrir le dépôt"),
                            Some("Voulez-vous remplacer le dépôt actuel ou l'ajouter dans un nouvel onglet ?"),
                        );
                        dlg.add_response("replace", "Remplacer");
                        dlg.add_response("add", "Ajouter");
                        dlg.set_default_response(Some("add"));
                        let state_c3 = state_for_recent.clone();
                        let commit_list_c3 = commit_list_rc.clone();
                        let details_c3 = details_rc.clone();
                        let side_c3 = side_rc.clone();
                        let window_c3 = window_rc.clone();
                        let load_more_c3 = load_more_rc.clone();
                        let search_entry_c3 = search_entry_rc.clone();
                        let starred_c3 = starred_rc.clone();
                        let prev_c3 = prev_rc.clone();
                        let star_db_c3 = star_db_rc.clone();
                        let status_lbl_c3 = status_lbl_rc.clone();
                        let recent_db_c3 = recent_db_rc.clone();
                        let refresh_recents_menu_c3 = refresh_recents_menu_rc.clone();
                        let run_repo_op_c3 = run_repo_op_rc.clone();
                        let rebuild_tabs_c3 = rebuild_tabs_rc.clone();
                        let status_sync_button_rc2 = status_sync_button_rc.clone();
                        dlg.connect_response(None, move |d: &adw::MessageDialog, resp: &str| {
                            d.hide();
                            if resp == "replace" {
                                {
                                    let mut st = state_c3.borrow_mut();
                                    st.repos.clear();
                                    st.repos.push(p.clone());
                                    st.current_idx = Some(0);
                                }
                            } else {
                                let mut st = state_c3.borrow_mut();
                                if !st.repos.iter().any(|x| x == &p) {
                                    st.repos.push(p.clone());
                                }
                                st.current_idx = st.repos.iter().position(|x| x == &p);
                            }
                            rebuild_tabs_c3();
                            load_repo(
                                &p,
                                &state_c3,
                                &commit_list_c3,
                                &details_c3,
                                &side_c3,
                                &window_c3,
                                &load_more_c3,
                                &search_entry_c3,
                                &starred_c3,
                                &prev_c3,
                                &star_db_c3,
                                &status_lbl_c3,
                                &status_sync_button_rc2,
                            );
                            let _ = recent_db_c3.touch(&p);
                            refresh_recents_menu_c3();
                            run_repo_op_c3("Fetch", Box::new(|repo| repo.fetch().map_err(|e| e.into())));
                        });
                        dlg.present();
                    } else {
                        {
                            let mut st = state_for_recent.borrow_mut();
                            if !st.repos.iter().any(|x| x == &p) {
                                st.repos.push(p.clone());
                            }
                            st.current_idx = st.repos.iter().position(|x| x == &p);
                        }
                        rebuild_tabs_rc();
                        load_repo(
                            &p,
                            &state_for_recent,
                            &commit_list_rc,
                            &details_rc,
                            &side_rc,
                            &window_rc,
                            &load_more_rc,
                            &search_entry_rc,
                            &starred_rc,
                            &prev_rc,
                            &star_db_rc,
                            &status_lbl_rc,
                            &status_sync_button_rc,
                        );
                        let _ = recent_db_rc.touch(&p);
                        refresh_recents_menu_rc();
                        run_repo_op_rc("Fetch", Box::new(|repo| repo.fetch().map_err(|e| e.into())));
                    }
                }
            }
        });
        app.add_action(&act_open_recent);

        // Clear recents
        let recent_db_c2 = recent_db.clone();
        let refresh_recents_menu_c2 = refresh_recents_menu.clone();
        let act_clear_recents = gio::SimpleAction::new("clear-recents", None);
        act_clear_recents.connect_activate(move |_, _| {
            if recent_db_c2.clear().is_ok() {
                refresh_recents_menu_c2();
            }
        });
        app.add_action(&act_clear_recents);

        let run_repo_op_c = run_repo_op.clone();
        let act_fetch = gio::SimpleAction::new("fetch", None);
        act_fetch.connect_activate(move |_, _| {
            run_repo_op_c("Fetch", Box::new(|repo| repo.fetch().map_err(|e| e.into())));
        });
        app.add_action(&act_fetch);

        let run_repo_op_c = run_repo_op.clone();
        let act_pull = gio::SimpleAction::new("pull", None);
        act_pull.connect_activate(move |_, _| {
            run_repo_op_c("Pull", Box::new(|repo| repo.pull().map_err(|e| e.into())));
        });
        app.add_action(&act_pull);

        let run_repo_op_c = run_repo_op.clone();
        let act_push = gio::SimpleAction::new("push", None);
        act_push.connect_activate(move |_, _| {
            // Placeholder using pull until push is implemented in core
            run_repo_op_c("Push", Box::new(|repo| repo.pull().map_err(|e| e.into())));
        });
        app.add_action(&act_push);

        let run_repo_op_c = run_repo_op.clone();
        let act_stash = gio::SimpleAction::new("stash", None);
        act_stash.connect_activate(move |_, _| {
            run_repo_op_c("Stash", Box::new(|repo| {
                let mut r = RepoHandle::open(&repo.path)?;
                r.stash("WIP").map_err(|e| e.into())
            }));
        });
        app.add_action(&act_stash);

        let act_quit = gio::SimpleAction::new("quit", None);
        act_quit.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() { app.quit(); }
        });
        app.add_action(&act_quit);
    }

    // View/dark mode action and dynamic menu label
    {
        let view_menu_c = view_menu.clone();
        let act_toggle_dark = gio::SimpleAction::new("toggle-dark", None);
        // Track current theme locally
        let is_dark = Rc::new(RefCell::new(true));
        act_toggle_dark.connect_activate(move |_, _| {
            let now = !*is_dark.borrow();
            *is_dark.borrow_mut() = now;
            let style_manager = StyleManager::default();
            if now {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            }

            // Update the View menu label dynamically
            // index 0 is our theme entry
            view_menu_c.remove(0);
            if now {
                view_menu_c.insert(0, Some("Switch to light theme"), Some("app.toggle-dark"));
            } else {
                view_menu_c.insert(0, Some("Switch to dark theme"), Some("app.toggle-dark"));
            }
        });
        app.add_action(&act_toggle_dark);
    }

    // Load more commits
    {
        let state = state.clone();
        let commit_list_c = commit_list.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();
        load_more_button.connect_clicked(move |_| {
            let (path_opt, offset) = {
                let st = state.borrow();
                let p = st.repo_path.clone();
                let off = p.as_ref().and_then(|k| st.loaded_by_repo.get(k)).copied().unwrap_or(0);
                (p, off)
            };
            if let Some(path) = path_opt {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Ok(commits) = repo.list_commits_paginated(offset, PAGE_SIZE) {
                        if commits.is_empty() {
                            load_more_c.set_sensitive(false);
                        } else {
                            commit_list_c.append(commits.clone());
                            {
                                let mut st = state.borrow_mut();
                                let entry = st.loaded_by_repo.entry(path.clone()).or_insert(0);
                                *entry += commits.len();
                            }
                            load_more_c.set_sensitive(commits.len() == PAGE_SIZE);
                            commit_list_c.filter(&search_entry_c.text());
                        }
                    }
                }
            }
        });
    }

    // Selection → details
    {
        let details_c = details.clone();
        let state_for_select = state.clone();
        commit_list.connect_on_select(move |oid| {
            if let Some(path) = state_for_select.borrow().repo_path.clone() {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Ok((info, message, diff)) = repo.get_commit_details(oid) {
                        details_c.show_commit(&info, &message, &diff);
                    }
                }
            }
        });
    }

    // Commit list context menu actions
    {
        let details_c = details.clone();
        let state_c = state.clone();
        let run_repo_op_c = run_repo_op.clone();
        commit_list.on_action(move |action, oid| {
            // Helper: open details for a given commit
            let open_details = || {
                if let Some(path) = state_c.borrow().repo_path.clone() {
                    if let Ok(repo) = RepoHandle::open(&path) {
                        if let Ok((info, message, diff)) = repo.get_commit_details(&oid) {
                            details_c.show_commit(&info, &message, &diff);
                        }
                    }
                }
            };
            match action {
                CommitContextAction::OpenChanges | CommitContextAction::InspectDetails => {
                    open_details();
                }
                CommitContextAction::CopySha => {
                    if let Some(cb) = Display::default().map(|d| d.clipboard()) {
                        cb.set_text(&oid);
                    }
                }
                CommitContextAction::CopyMessage | CommitContextAction::Copy => {
                    if let Some(path) = state_c.borrow().repo_path.clone() {
                        if let Ok(repo) = RepoHandle::open(&path) {
                            if let Ok((info, _msg, _)) = repo.get_commit_details(&oid) {
                                let text = if matches!(action, CommitContextAction::CopyMessage) {
                                    _msg
                                } else {
                                    format!("[{}] {}", info.short_id, info.summary)
                                };
                                if let Some(cb) = Display::default().map(|d| d.clipboard()) {
                                    cb.set_text(&text);
                                }
                            }
                        }
                    }
                }
                CommitContextAction::CopyPatch => {
                    let path_opt = { state_c.borrow().repo_path.clone() };
                    if let Some(path) = path_opt {
                        if let Ok(repo) = RepoHandle::open(&path) {
                            if let Ok(patch) = repo.get_commit_patch_text(&oid) {
                                if let Some(cb) = Display::default().map(|d| d.clipboard()) {
                                    cb.set_text(&patch);
                                }
                            }
                        }
                    }
                }
                CommitContextAction::CreateBranch => {
                    let dialog = adw::MessageDialog::builder()
                        .body("Nom de la branche à créer à ce commit:")
                        .heading("Créer une branche")
                        .extra_child(&{
                            let entry = gtk::Entry::new();
                            entry.set_hexpand(true);
                            entry.set_widget_name("branch_entry");
                            entry
                        })
                        .build();
                    dialog.add_response("cancel", "Annuler");
                    dialog.add_response("ok", "Créer");
                    dialog.set_default_response(Some("ok"));
                    let run_repo_op_c2 = run_repo_op_c.clone();
                    dialog.connect_response(None, move |d: &adw::MessageDialog, resp: &str| {
                        if resp == "ok" {
                            if let Some(entry) = d
                                .extra_child()
                                .and_then(|w| w.downcast::<gtk::Entry>().ok())
                            {
                                let name = entry.text().to_string();
                                if !name.is_empty() {
                                    let oid_c = oid.clone();
                                    run_repo_op_c2("Create Branch", Box::new(move |repo| {
                                        repo.create_branch_at(&name, &oid_c).map_err(|e| e.into())
                                    }));
                                }
                            }
                        }
                    });
                    dialog.present();
                }
                CommitContextAction::CreateTag => {
                    let dialog = adw::MessageDialog::builder()
                        .body("Nom du tag à créer à ce commit:")
                        .heading("Créer un tag")
                        .extra_child(&{
                            let entry = gtk::Entry::new();
                            entry.set_hexpand(true);
                            entry.set_widget_name("tag_entry");
                            entry
                        })
                        .build();
                    dialog.add_response("cancel", "Annuler");
                    dialog.add_response("ok", "Créer");
                    dialog.set_default_response(Some("ok"));
                    let run_repo_op_c2 = run_repo_op_c.clone();
                    dialog.connect_response(None, move |d: &adw::MessageDialog, resp: &str| {
                        if resp == "ok" {
                            if let Some(entry) = d
                                .extra_child()
                                .and_then(|w| w.downcast::<gtk::Entry>().ok())
                            {
                                let name = entry.text().to_string();
                                if !name.is_empty() {
                                    let oid_c = oid.clone();
                                    run_repo_op_c2("Create Tag", Box::new(move |repo| {
                                        repo.create_tag(&name, &oid_c).map_err(|e| e.into())
                                    }));
                                }
                            }
                        }
                    });
                    dialog.present();
                }
                CommitContextAction::CreatePatch => {
                    // Save patch to a file (quick path without dialog): in repository path
                    if let Some(path) = state_c.borrow().repo_path.clone() {
                        if let Ok(repo) = RepoHandle::open(&path) {
                            if let Ok(patch) = repo.get_commit_patch_text(&oid) {
                                let default = format!("{}.patch", &oid[..7.min(oid.len())]);
                                let target = std::path::Path::new(&path).join(default);
                                let _ = std::fs::write(&target, patch);
                                let info = adw::MessageDialog::new(
                                    None::<&adw::ApplicationWindow>,
                                    Some("Patch créé"),
                                    Some(&format!("Enregistré dans: {}", target.display())),
                                );
                                info.add_response("ok", "OK");
                                info.present();
                            }
                        }
                    }
                }
                CommitContextAction::ResetTo => {
                    let run_repo_op_c2 = run_repo_op_c.clone();
                    let oid_c = oid.clone();
                    let dlg = adw::MessageDialog::new(
                        None::<&adw::ApplicationWindow>,
                        Some("Confirmer le reset"),
                        Some("Réinitialiser la branche courante sur ce commit (hard reset) ?"),
                    );
                    dlg.add_response("cancel", "Annuler");
                    dlg.add_response("ok", "Reset");
                    dlg.connect_response(None, move |_d: &adw::MessageDialog, resp: &str| {
                        if resp == "ok" {
                            let value = oid_c.clone();
                            run_repo_op_c2("Reset", Box::new(move |repo| repo.reset_hard_to(&value).map_err(|e| e.into())));
                        }
                    });
                    dlg.present();
                }
                CommitContextAction::ResetToPrevious => {
                    let oid_c = oid.clone();
                    let run_repo_op_c2 = run_repo_op_c.clone();
                    run_repo_op_c2("Reset", Box::new(move |repo| repo.reset_hard_to_parent(&oid_c).map_err(|e| e.into())));
                }
                CommitContextAction::SwitchTo => {
                    let oid_c = oid.clone();
                    let run_repo_op_c2 = run_repo_op_c.clone();
                    run_repo_op_c2("Checkout", Box::new(move |repo| repo.checkout_commit(&oid_c).map_err(|e| e.into())));
                }
                CommitContextAction::OpenOnRemote => {
                    if let Some(path) = state_c.borrow().repo_path.clone() {
                        if let Ok(repo) = RepoHandle::open(&path) {
                            if let Some(url) = repo.commit_remote_url(&oid) {
                                if let Some(cb) = Display::default().map(|d| d.clipboard()) {
                                    cb.set_text(&url);
                                }
                                let info = adw::MessageDialog::new(
                                    None::<&adw::ApplicationWindow>,
                                    Some("Lien copié"),
                                    Some(&url),
                                );
                                info.add_response("ok", "OK");
                                info.present();
                            }
                        }
                    }
                }
                // Not yet implemented actions → friendly notification
                CommitContextAction::Revert
                | CommitContextAction::AiRebasePreview
                | CommitContextAction::RebaseOnto
                | CommitContextAction::ExplainChanges
                | CommitContextAction::CompareToFromHead
                | CommitContextAction::CompareWorkingTreeToHere
                | CommitContextAction::Share => {
                    let info = adw::MessageDialog::new(
                        None::<&adw::ApplicationWindow>,
                        Some("Bientôt disponible"),
                        Some("Cette action n'est pas encore implémentée."),
                    );
                    info.add_response("ok", "OK");
                    info.present();
                }
            }
        });
    }

    // Build initial tabbar (placeholder if needed) and populate Recents submenu
    (rebuild_tabs)();
    refresh_recents_menu();

    // Démarrage: tenter d'ouvrir --repo PATH si passé en argument
    if let Some(path) = std::env::args().skip_while(|a| a != "--repo").nth(1) {
        load_repo(
            &path,
            &state,
            &commit_list,
            &details,
            &side_panel,
            &window,
            &load_more_button,
            &search_entry,
            &starred,
            &prev_stars,
            &star_db,
            &status_branch_label,
            &status_sync_button,
        );
        // Assure l'affichage de l'onglet dès le premier dépôt
        (rebuild_tabs)();
        let _ = recent_db.touch(&path);
        refresh_recents_menu();
        let run_repo_op_c = run_repo_op.clone();
        run_repo_op_c("Fetch", Box::new(|repo| repo.fetch().map_err(|e| e.into())));
    }

    window.present();
}
