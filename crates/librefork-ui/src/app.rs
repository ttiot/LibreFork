use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, StyleManager};
use gtk::Orientation;
use gtk4 as gtk;
use gtk4::{gdk::Display, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use librefork_core::RepoHandle;
use std::cell::RefCell;
use std::rc::Rc;

use crate::widgets::{
    commit_details::CommitDetails, commit_list::CommitList, side_panel::SidePanel,
};
use crate::starred::StarredItem;
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

    // Header
    let title_label = gtk::Label::new(Some("LibreFork"));
    let header = HeaderBar::builder().title_widget(&title_label).build();
    let open_button = gtk::Button::with_label("Ouvrir un dépôt…");
    open_button.add_css_class("suggested-action");
    header.pack_start(&open_button);

    let branch_combo = gtk::ComboBoxText::new();
    header.pack_start(&branch_combo);

    let refresh_button = gtk::Button::with_label("Rafraîchir");
    header.pack_end(&refresh_button);

    let fetch_button = gtk::Button::with_label("Fetch");
    header.pack_end(&fetch_button);

    let pull_button = gtk::Button::with_label("Pull");
    header.pack_end(&pull_button);

    let push_button = gtk::Button::with_label("Push");
    header.pack_end(&push_button);

    let stash_button = gtk::Button::with_label("Stash");
    header.pack_end(&stash_button);

    let theme_switch = gtk::Switch::new();
    header.pack_end(&theme_switch);

    // let settings = gtk::Settings::default().expect("Could not get default settings");
    let style_manager = StyleManager::default();
    // settings.set_gtk_application_prefer_dark_theme(true);
    theme_switch.set_active(true);
    {
        // let settings = settings.clone();
        theme_switch.connect_active_notify(move |sw| {
            if sw.is_active() {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            }
        });
    }

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

    let starred: Rc<RefCell<HashSet<StarredItem>>> = Rc::new(RefCell::new(HashSet::new()));
    let commit_list = CommitList::new(starred.clone());
    let details = CommitDetails::new();
    let side_panel = SidePanel::new(starred.clone());
    {
        let side_c = side_panel.clone();
        commit_list.on_star_changed(move || {
            side_c.reload();
        });
    }

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
    content.append(&header);
    content.append(&outer);

    window.set_content(Some(&content));

    {
        let commit_list_c = commit_list.clone();
        search_entry.connect_search_changed(move |entry| {
            commit_list_c.filter(&entry.text());
        });
    }

    // State
    #[derive(Default, Clone)]
    struct State {
        repo_path: Option<String>,
        loaded: usize,
    }
    let state = Rc::new(RefCell::new(State::default()));
    const PAGE_SIZE: usize = 100;

    fn load_repo(
        path: &str,
        state: &Rc<RefCell<State>>,
        commit_list: &CommitList,
        details: &CommitDetails,
        side: &SidePanel,
        branch_combo: &gtk::ComboBoxText,
        title_label: &gtk::Label,
        load_more: &gtk::Button,
        search_entry: &gtk::SearchEntry,
    ) {
        match RepoHandle::open(path) {
            Ok(repo) => {
                if let Ok(head) = repo.head() {
                    if let Some(name) = head {
                        title_label.set_text(&format!("LibreFork - {}", name));
                    }
                }

                if let Ok(branches) = repo.list_branches() {
                    branch_combo.remove_all();
                    let head_name = repo.head().ok().flatten();
                    let mut active = None;
                    for (i, b) in branches.iter().enumerate() {
                        branch_combo.append_text(b);
                        if head_name.as_ref().map(|h| h == b).unwrap_or(false) {
                            active = Some(i as u32);
                        }
                    }
                    if let Some(idx) = active {
                        branch_combo.set_active(Some(idx));
                    }
                }

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
                st.loaded = 0;

                if let Ok(commits) = repo.list_commits_paginated(0, PAGE_SIZE) {
                    commit_list.load(commits.clone());
                    details.clear();
                    st.loaded = commits.len();
                    load_more.set_sensitive(commits.len() == PAGE_SIZE);
                    search_entry.set_text("");
                    commit_list.filter("");
                }
            }
            Err(err) => eprintln!("Erreur d'ouverture du dépôt: {err}"),
        }
    }

    // Interactions
    {
        let state = state.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let side_c = side_panel.clone();
        let branch_combo_c = branch_combo.clone();
        let title_label_c = title_label.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();
        refresh_button.connect_clicked(move |_| {
            let path_opt = { state.borrow().repo_path.clone() };
            if let Some(path) = path_opt {
                load_repo(
                    &path,
                    &state,
                    &commit_list_c,
                    &details_c,
                    &side_c,
                    &branch_combo_c,
                    &title_label_c,
                    &load_more_c,
                    &search_entry_c,
                );
            }
        });
    }

    {
        let state = state.clone();
        fetch_button.connect_clicked(move |_| {
            if let Some(path) = state.borrow().repo_path.clone() {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Err(err) = repo.fetch() {
                        eprintln!("Fetch error: {err}");
                    }
                }
            }
        });
    }

    {
        let state = state.clone();
        pull_button.connect_clicked(move |_| {
            if let Some(path) = state.borrow().repo_path.clone() {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Err(err) = repo.pull() {
                        eprintln!("Pull error: {err}");
                    }
                }
            }
        });
    }

    {
        let state = state.clone();

        push_button.connect_clicked(move |_| {
            if let Some(path) = state.borrow().repo_path.clone() {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Err(err) = repo.pull() {
                        eprintln!("Push error: {err}");
                    }
                }
            }
        });
    }

    {
        let state = state.clone();
        stash_button.connect_clicked(move |_| {
            if let Some(path) = state.borrow().repo_path.clone() {
                if let Ok(mut repo) = RepoHandle::open(&path) {
                    if let Err(err) = repo.stash("WIP") {
                        eprintln!("Stash error: {err}");
                    }
                }
            }
        });
    }

    {
        let state_for_dialog = state.clone();
        let window = window.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let side_c_cloned = side_panel.clone(); // Clone du Rc ici
        let branch_combo_c = branch_combo.clone();
        let title_label_c = title_label.clone();
        let load_more_c = load_more_button.clone();
        let search_entry_c = search_entry.clone();

        // Holder pour garder le dialog vivant jusqu'à la réponse
        let dialog_holder: Rc<RefCell<Option<gtk::FileChooserNative>>> =
            Rc::new(RefCell::new(None));

        open_button.connect_clicked(move |_| {
            let dialog = gtk::FileChooserNative::builder()
                .title("Ouvrir un dépôt Git")
                .action(gtk::FileChooserAction::SelectFolder)
                .transient_for(&window) // parent
                .modal(true)
                .build();

            // conservez une ref forte
            *dialog_holder.borrow_mut() = Some(dialog.clone());

            dialog.connect_response({
                let state_for_dialog_cb = state_for_dialog.clone();
                let commit_list_c = commit_list_c.clone();
                let details_c = details_c.clone();
                let side_c_cloned = side_c_cloned.clone(); // Cloner ici
                let branch_combo_c2 = branch_combo_c.clone();
                let title_label_c2 = title_label_c.clone();
                let load_more_c2 = load_more_c.clone();
                let search_entry_c2 = search_entry_c.clone();
                let holder = dialog_holder.clone();

                move |dlg, resp| {
                    holder.borrow_mut().take();

                    if resp == gtk::ResponseType::Accept {
                        if let Some(file) = dlg.file() {
                            if let Some(path) = file.path() {
                                load_repo(
                                    path.to_string_lossy().as_ref(),
                                    &state_for_dialog_cb,
                                    &commit_list_c,
                                    &details_c,
                                    &side_c_cloned, // Utilisation du clone ici
                                    &branch_combo_c2,
                                    &title_label_c2,
                                    &load_more_c2,
                                    &search_entry_c2,
                                );
                            }
                        }
                    }
                }
            });

            dialog.show();
        });
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
                (st.repo_path.clone(), st.loaded)
            };
            if let Some(path) = path_opt {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Ok(commits) = repo.list_commits_paginated(offset, PAGE_SIZE) {
                        if commits.is_empty() {
                            load_more_c.set_sensitive(false);
                        } else {
                            commit_list_c.append(commits.clone());
                            state.borrow_mut().loaded += commits.len();
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

    // Démarrage: tenter d'ouvrir --repo PATH si passé en argument
    if let Some(path) = std::env::args().skip_while(|a| a != "--repo").nth(1) {
        load_repo(
            &path,
            &state,
            &commit_list,
            &details,
            &side_panel,
            &branch_combo,
            &title_label,
            &load_more_button,
            &search_entry,
        );
    }

    window.present();
}
