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
    let header = HeaderBar::builder()
        .title_widget(&gtk::Label::new(Some("LibreFork")))
        .build();
    let open_button = gtk::Button::with_label("Ouvrir un dépôt…");
    open_button.add_css_class("suggested-action");
    header.pack_start(&open_button);

    let refresh_button = gtk::Button::with_label("Rafraîchir");
    header.pack_end(&refresh_button);

    // Main layout
    let paned = gtk::Paned::builder()
        .orientation(Orientation::Horizontal)
        .start_child(&gtk::Label::new(None))
        .end_child(&gtk::Label::new(None))
        .wide_handle(true)
        .build();

    let left = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    let right = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let commit_list = CommitList::new();
    let details = CommitDetails::new();

    left.set_child(Some(commit_list.widget()));
    right.set_child(Some(details.widget()));

    paned.set_start_child(Some(&left));
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
    }
    let state = Rc::new(RefCell::new(State::default()));

    // Interactions
    {
        let state = state.clone();
        let commit_list_c = commit_list.clone();
        let details_c = details.clone();
        refresh_button.connect_clicked(move |_| {
            if let Some(path) = state.borrow().repo_path.clone() {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Ok(commits) = repo.list_commits(500) {
                        commit_list_c.load(commits);
                        details_c.clear();
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
                let holder = dialog_holder.clone();

                move |dlg, resp| {
                    // relâche la ref forte pour permettre la destruction
                    holder.borrow_mut().take();

                    if resp == gtk::ResponseType::Accept {
                        if let Some(file) = dlg.file() {
                            if let Some(path) = file.path() {
                                match librefork_core::RepoHandle::open(
                                    path.to_string_lossy().as_ref(),
                                ) {
                                    Ok(repo) => {
                                        state_for_dialog_cb.borrow_mut().repo_path =
                                            Some(path.to_string_lossy().to_string());
                                        if let Ok(commits) = repo.list_commits(500) {
                                            commit_list_c.load(commits);
                                            details_c.clear();
                                        }
                                    }
                                    Err(err) => eprintln!("Erreur d'ouverture du dépôt: {err}"),
                                }
                            }
                        }
                    }
                }
            });

            dialog.show();
        });
    }

    // Selection → details
    {
        let details_c = details.clone();
        let state_for_select = state.clone();
        commit_list.connect_on_select(move |oid| {
            if let Some(path) = state_for_select.borrow().repo_path.clone() {
                if let Ok(repo) = RepoHandle::open(&path) {
                    if let Ok((info, message)) = repo.get_commit_details(oid) {
                        details_c.show_commit(&info, &message);
                    }
                }
            }
        });
    }

    // Démarrage: tenter d'ouvrir --repo PATH si passé en argument
    if let Some(path) = std::env::args().skip_while(|a| a != "--repo").nth(1) {
        if let Ok(repo) = RepoHandle::open(&path) {
            state.borrow_mut().repo_path = Some(path.clone());
            if let Ok(commits) = repo.list_commits(500) {
                commit_list.load(commits);
            }
        }
    }

    window.present();
}
