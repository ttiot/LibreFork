use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar};
use gtk::Orientation;
use gtk4 as gtk;
use librefork_core::RepoHandle;
use std::cell::RefCell;
use std::rc::Rc;

use crate::widgets::{commit_details::CommitDetails, commit_list::CommitList};

pub fn build_ui(app: &Application) {
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

    // Main layout
    let paned = gtk::Paned::builder()
        .orientation(Orientation::Horizontal)
        .start_child(&gtk::Label::new(None))
        .end_child(&gtk::Label::new(None))
        .wide_handle(true)
        .build();

    let left_scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    let right = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let commit_list = CommitList::new();
    let details = CommitDetails::new();

    left_scrolled.set_child(Some(commit_list.widget()));
    right.set_child(Some(details.widget()));

    let load_more_button = gtk::Button::with_label("Charger plus");
    let left_box = gtk::Box::new(Orientation::Vertical, 0);
    left_box.append(&left_scrolled);
    left_box.append(&load_more_button);

    paned.set_start_child(Some(&left_box));
    paned.set_end_child(Some(&right));
    paned.set_position(420);

    let content = gtk::Box::new(Orientation::Vertical, 0);
    content.append(&header);
    content.append(&paned);

    window.set_content(Some(&content));

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
        branch_combo: &gtk::ComboBoxText,
        title_label: &gtk::Label,
        load_more: &gtk::Button,
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

                let mut st = state.borrow_mut();
                st.repo_path = Some(path.to_string());
                st.loaded = 0;

                if let Ok(commits) = repo.list_commits_paginated(0, PAGE_SIZE) {
                    commit_list.load(commits.clone());
                    details.clear();
                    st.loaded = commits.len();
                    load_more.set_sensitive(commits.len() == PAGE_SIZE);
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
        let branch_combo_c = branch_combo.clone();
        let title_label_c = title_label.clone();
        let load_more_c = load_more_button.clone();
        refresh_button.connect_clicked(move |_| {
            let path_opt = { state.borrow().repo_path.clone() };
            if let Some(path) = path_opt {
                load_repo(
                    &path,
                    &state,
                    &commit_list_c,
                    &details_c,
                    &branch_combo_c,
                    &title_label_c,
                    &load_more_c,
                );
            }
        });
    }

    {
        let state_for_dialog = state.clone();
        let window = window.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        let branch_combo_c = branch_combo.clone();
        let title_label_c = title_label.clone();
        let load_more_c = load_more_button.clone();

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
                let branch_combo_c2 = branch_combo_c.clone();
                let title_label_c2 = title_label_c.clone();
                let load_more_c2 = load_more_c.clone();
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
                                    &branch_combo_c2,
                                    &title_label_c2,
                                    &load_more_c2,
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
            &branch_combo,
            &title_label,
            &load_more_button,
        );
    }

    window.present();
}
