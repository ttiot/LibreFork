use adw::prelude::*;
use chrono::{DateTime, Local, Locale};
use gtk::Orientation;
use gtk4 as gtk;
use gtk4::{cairo, pango};
use gtk::gdk;
use librefork_core::CommitInfo;
use crate::starred::StarredItem;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitContextAction {
    Revert,
    ResetTo,
    ResetToPrevious,
    AiRebasePreview,
    RebaseOnto,
    SwitchTo,
    CreateBranch,
    CreatePatch,
    CreateTag,
    ExplainChanges,
    OpenChanges,
    InspectDetails,
    OpenOnRemote,
    CompareToFromHead,
    CompareWorkingTreeToHere,
    CopyPatch,
    Share,
    Copy,
    CopySha,
    CopyMessage,
}

#[derive(Clone)]
pub struct CommitList {
    root: gtk::Box,
    list: gtk::ListBox,
    commits: Rc<RefCell<Vec<CommitInfo>>>,
    on_select: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    starred: Rc<RefCell<HashSet<StarredItem>>>,
    on_star_changed: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    on_action: Rc<RefCell<Option<Box<dyn Fn(CommitContextAction, String)>>>>,
}

#[derive(Clone)]
struct GraphRowData {
    active_before: Vec<Option<String>>,
    node_lane: usize,
    parent_lanes: Vec<usize>,
    lane_count: usize,
    // Lanes (other than the node lane) that currently carry this commit
    // and should visually connect from the top half into the node.
    incoming_lanes: Vec<usize>,
}

const LANE_COLORS: [(f64, f64, f64); 8] = [
    (0.89, 0.10, 0.11),
    (0.22, 0.49, 0.72),
    (0.20, 0.63, 0.17),
    (0.60, 0.31, 0.64),
    (1.00, 0.50, 0.00),
    (0.65, 0.34, 0.16),
    (0.97, 0.51, 0.75),
    (0.13, 0.70, 0.67),
];

impl CommitList {
    pub fn new(starred: Rc<RefCell<HashSet<StarredItem>>>) -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_vexpand(true);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_vexpand(true);

        root.append(&list);

        let on_select: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));
        let commits: Rc<RefCell<Vec<CommitInfo>>> = Rc::new(RefCell::new(Vec::new()));
        let on_star_changed: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let on_action: Rc<RefCell<Option<Box<dyn Fn(CommitContextAction, String)>>>> = Rc::new(RefCell::new(None));

        let cb = on_select.clone();
        list.connect_row_selected(move |_, row| {
            if let (Some(row), Some(cb)) = (row, &*cb.borrow()) {
                let oid = row.widget_name().to_string();
                if !oid.is_empty() {
                    cb(&oid);
                }
            }
        });

        // Global right-click on the list as a fallback to ensure menu appears
        {
            let list_c = list.clone();
            let on_action_c = on_action.clone();
            let click = gtk::GestureClick::new();
            click.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
            // Let row-specific handlers run first; this is a fallback only
            click.set_propagation_phase(gtk::PropagationPhase::Bubble);
            click.connect_pressed(move |g, _n, x, y| {
                if g.current_button() != 3 { return; }
                if let Some(row) = list_c.row_at_y(y as i32) {
                    let oid = row.widget_name().to_string();
                    if oid.is_empty() { return; }
                    // Claim the sequence so ancestors don't also open a menu
                    g.set_state(gtk::EventSequenceState::Claimed);

                    // Build a modern-looking, left-aligned popover menu
                    let pop = gtk::Popover::new();
                    pop.add_css_class("context-menu");
                    let bx = gtk::Box::new(Orientation::Vertical, 0);
                    bx.set_margin_top(4);
                    bx.set_margin_bottom(4);

                    let mut add_btn = |label: &str, action: CommitContextAction| {
                        let btn = gtk::Button::new();
                        btn.add_css_class("flat");
                        btn.add_css_class("context-menu-btn");
                        btn.set_halign(gtk::Align::Fill);
                        btn.set_hexpand(true);
                        let lbl = gtk::Label::new(Some(label));
                        lbl.set_xalign(0.0);
                        btn.set_child(Some(&lbl));
                        let on_action_c = on_action_c.clone();
                        let oid = oid.clone();
                        let pop_c = pop.clone();
                        btn.connect_clicked(move |_| {
                            if let Some(cb) = &*on_action_c.borrow() { cb(action, oid.clone()); }
                            pop_c.popdown();
                        });
                        bx.append(&btn);
                    };
                    add_btn("Revert Commit…", CommitContextAction::Revert);
                    add_btn("Reset Current Branch to Commit…", CommitContextAction::ResetTo);
                    add_btn("Reset Current Branch to Previous Commit…", CommitContextAction::ResetToPrevious);
                    add_btn("AI Rebase Current Branch onto Commit (Preview)…", CommitContextAction::AiRebasePreview);
                    add_btn("Rebase Current Branch onto Commit…", CommitContextAction::RebaseOnto);
                    add_btn("Switch to Commit…", CommitContextAction::SwitchTo);
                    bx.append(&gtk::Separator::new(Orientation::Horizontal));
                    add_btn("Create Branch…", CommitContextAction::CreateBranch);
                    add_btn("Create Patch…", CommitContextAction::CreatePatch);
                    add_btn("Create Tag…", CommitContextAction::CreateTag);
                    bx.append(&gtk::Separator::new(Orientation::Horizontal));
                    add_btn("Explain Changes (Preview)", CommitContextAction::ExplainChanges);
                    add_btn("Open Changes", CommitContextAction::OpenChanges);
                    add_btn("Inspect Details", CommitContextAction::InspectDetails);
                    add_btn("Open Commit on Remote", CommitContextAction::OpenOnRemote);
                    add_btn("Compare to/from HEAD", CommitContextAction::CompareToFromHead);
                    add_btn("Compare Working Tree to Here", CommitContextAction::CompareWorkingTreeToHere);
                    add_btn("Copy Changes (Patch)", CommitContextAction::CopyPatch);
                    bx.append(&gtk::Separator::new(Orientation::Horizontal));
                    add_btn("Share", CommitContextAction::Share);
                    add_btn("Copy", CommitContextAction::Copy);
                    add_btn("Copy SHA", CommitContextAction::CopySha);
                    add_btn("Copy Message", CommitContextAction::CopyMessage);

                    pop.set_child(Some(&bx));
                    // Parent to the list to avoid coordinate transforms
                    pop.set_parent(&list_c);
                    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                    pop.popup();
                }
            });
            list.add_controller(click);
        }

        Self {
            root,
            list,
            commits,
            on_select,
            starred,
            on_star_changed,
            on_action,
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    pub fn on_star_changed(&self, cb: impl Fn() + 'static) {
        *self.on_star_changed.borrow_mut() = Some(Box::new(cb));
    }

    pub fn on_action(&self, cb: impl Fn(CommitContextAction, String) + 'static) {
        *self.on_action.borrow_mut() = Some(Box::new(cb));
    }

    fn row_from_commit(
        c: &CommitInfo,
        g: &GraphRowData,
        starred: Rc<RefCell<HashSet<StarredItem>>>,
        on_star_changed: Rc<RefCell<Option<Box<dyn Fn()>>>>,
        on_action: Rc<RefCell<Option<Box<dyn Fn(CommitContextAction, String)>>>>,
    ) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        row.set_height_request(24);
        row.set_margin_top(0);
        row.set_margin_bottom(0);
        row.add_css_class("commit-row");

        let row_box = gtk::Box::new(Orientation::Horizontal, 8);
        row_box.set_margin_top(0);
        row_box.set_margin_bottom(0);
        row_box.set_margin_start(8);
        row_box.set_margin_end(8);

        let star_label = gtk::Label::new(Some("☆"));
        star_label.set_width_chars(1);
        if starred
            .borrow()
            .contains(&StarredItem::Commit(c.oid.clone()))
        {
            star_label.set_text("★");
        }

        let graph = gtk::DrawingArea::new();
        graph.set_content_width((g.lane_count as i32) * 12);
        graph.set_content_height(24);
        let gd = g.clone();
        let colors = LANE_COLORS;
        graph.set_draw_func(move |_, cr: &cairo::Context, _w, h| {
            let h = h as f64;
            let lane_width = 12.0;
            let center = |lane: usize| lane_width / 2.0 + lane as f64 * lane_width;
            cr.set_line_width(2.0);

            for (lane, oid_opt) in gd.active_before.iter().enumerate() {
                if oid_opt.is_some() {
                    let (r, g, b) = colors[lane % colors.len()];
                    cr.set_source_rgb(r, g, b);
                    let x = center(lane);
                    cr.move_to(x, 0.0);
                    if lane == gd.node_lane {
                        // Node lane: straight down into the node (top half)
                        cr.line_to(x, h / 2.0);
                    } else if gd.incoming_lanes.contains(&lane) {
                        // Incoming lane carrying this commit: curve into the node (top half)
                        let x_node = center(gd.node_lane);
                        let y0 = 0.0;
                        let y_mid = h / 2.0;
                        cr.curve_to(
                            x,
                            (y0 + y_mid) / 2.0,
                            x_node,
                            (y0 + y_mid) / 2.0,
                            x_node,
                            y_mid,
                        );
                    } else {
                        // Unrelated active lane continues straight through
                        cr.line_to(x, h);
                    }
                    cr.stroke().ok();
                }
            }

            for &p_lane in &gd.parent_lanes {
                let (r, g, b) = colors[p_lane % colors.len()];
                cr.set_source_rgb(r, g, b);
                let x0 = center(gd.node_lane);
                let y0 = h / 2.0;
                let x1 = center(p_lane);
                let y1 = h;
                cr.move_to(x0, y0);
                if p_lane == gd.node_lane {
                    cr.line_to(x1, y1);
                } else {
                    cr.curve_to(x0, (y0 + y1) / 2.0, x1, (y0 + y1) / 2.0, x1, y1);
                }
                cr.stroke().ok();
            }

            let (r, g, b) = colors[gd.node_lane % colors.len()];
            cr.set_source_rgb(r, g, b);
            let x = center(gd.node_lane);
            cr.arc(x, h / 2.0, 3.0, 0.0, std::f64::consts::PI * 2.0);
            cr.fill().ok();
        });

        let message_box = gtk::Box::new(Orientation::Horizontal, 4);
        message_box.set_hexpand(true);

        let tag_box = gtk::Box::new(Orientation::Horizontal, 4);
        tag_box.set_valign(gtk::Align::Center);
        for r in &c.refs {
            let tag = gtk::Label::new(Some(r));
            tag.add_css_class("tag-label");
            tag_box.append(&tag);
        }
        message_box.append(&tag_box);

        let summary = gtk::Label::new(Some(&c.summary));
        summary.set_xalign(0.0);
        summary.set_ellipsize(pango::EllipsizeMode::End);
        summary.set_hexpand(true);
        summary.set_valign(gtk::Align::Center);
        let lowered = c.summary.to_lowercase();
        if lowered.contains("crash") {
            summary.add_css_class("commit-warning");
        }
        message_box.append(&summary);

        let author = gtk::Label::new(Some(&c.author));
        author.add_css_class("dim-label");
        author.set_xalign(0.0);
        author.set_valign(gtk::Align::Center);

        let hash = gtk::Label::new(Some(&c.short_id));
        hash.add_css_class("monospace");
        hash.add_css_class("dim-label");
        hash.set_xalign(0.0);
        hash.set_valign(gtk::Align::Center);

        let date_str = DateTime::parse_from_rfc3339(&c.time)
            .map(|dt| dt.with_timezone(&Local))
            .map(|dt| {
                let locale = std::env::var("LC_TIME")
                    .or_else(|_| std::env::var("LANG"))
                    .ok()
                    .and_then(|l| l.split('.').next().unwrap_or(&l).parse::<Locale>().ok())
                    .unwrap_or(Locale::en_US);
                dt.format_localized("%c", locale).to_string()
            })
            .unwrap_or_else(|_| c.time.clone());
        let date = gtk::Label::new(Some(&date_str));
        date.add_css_class("dim-label");
        date.set_xalign(0.0);
        date.set_valign(gtk::Align::Center);

        row_box.append(&star_label);
        row_box.append(&graph);
        row_box.append(&message_box);
        row_box.append(&author);
        row_box.append(&hash);
        row_box.append(&date);

        row.set_child(Some(&row_box));
        row.set_widget_name(&c.oid);

        let oid = c.oid.clone();
        let star_label_c = star_label.clone();
        let click = gtk::GestureClick::new();
        click.set_button(gdk::ffi::GDK_BUTTON_SECONDARY as u32);
        // Handle at the target so we can claim the sequence
        click.set_propagation_phase(gtk::PropagationPhase::Target);
        let row_c = row.clone();
        let on_action_cb = on_action.clone();
        click.connect_pressed(move |g, _, x, y| {
            if g.current_button() == 3 {
                // Claim this sequence to avoid ancestor fallback handling
                g.set_state(gtk::EventSequenceState::Claimed);
                let pop = gtk::Popover::new();
                pop.add_css_class("context-menu");
                let bx = gtk::Box::new(Orientation::Vertical, 0);
                bx.set_margin_top(4);
                bx.set_margin_bottom(4);

                // Helper to append a clickable button
                let mut add_btn = |label: &str, action: Option<CommitContextAction>| {
                    let btn = gtk::Button::new();
                    btn.add_css_class("flat");
                    btn.add_css_class("context-menu-btn");
                    btn.set_halign(gtk::Align::Fill);
                    btn.set_hexpand(true);
                    let lbl = gtk::Label::new(Some(label));
                    lbl.set_xalign(0.0);
                    btn.set_child(Some(&lbl));
                    if let Some(act) = action {
                        let oid = oid.clone();
                        let pop_c = pop.clone();
                        let on_action_cb = on_action_cb.clone();
                        btn.connect_clicked(move |_| {
                            if let Some(cb) = &*on_action_cb.borrow() {
                                cb(act, oid.clone());
                            }
                            pop_c.popdown();
                        });
                    }
                    bx.append(&btn);
                };

                // Build menu (mirrors screenshot order)
                add_btn("Revert Commit…", Some(CommitContextAction::Revert));
                add_btn(
                    "Reset Current Branch to Commit…",
                    Some(CommitContextAction::ResetTo),
                );
                add_btn(
                    "Reset Current Branch to Previous Commit…",
                    Some(CommitContextAction::ResetToPrevious),
                );
                add_btn(
                    "AI Rebase Current Branch onto Commit (Preview)…",
                    Some(CommitContextAction::AiRebasePreview),
                );
                add_btn(
                    "Rebase Current Branch onto Commit…",
                    Some(CommitContextAction::RebaseOnto),
                );
                add_btn("Switch to Commit…", Some(CommitContextAction::SwitchTo));

                bx.append(&gtk::Separator::new(Orientation::Horizontal));

                add_btn("Create Branch…", Some(CommitContextAction::CreateBranch));
                add_btn("Create Patch…", Some(CommitContextAction::CreatePatch));
                add_btn("Create Tag…", Some(CommitContextAction::CreateTag));

                bx.append(&gtk::Separator::new(Orientation::Horizontal));

                add_btn(
                    "Explain Changes (Preview)",
                    Some(CommitContextAction::ExplainChanges),
                );
                add_btn("Open Changes", Some(CommitContextAction::OpenChanges));
                add_btn("Inspect Details", Some(CommitContextAction::InspectDetails));
                add_btn(
                    "Open Commit on Remote",
                    Some(CommitContextAction::OpenOnRemote),
                );
                add_btn(
                    "Compare to/from HEAD",
                    Some(CommitContextAction::CompareToFromHead),
                );
                add_btn(
                    "Compare Working Tree to Here",
                    Some(CommitContextAction::CompareWorkingTreeToHere),
                );
                add_btn(
                    "Copy Changes (Patch)",
                    Some(CommitContextAction::CopyPatch),
                );

                bx.append(&gtk::Separator::new(Orientation::Horizontal));

                add_btn("Share", Some(CommitContextAction::Share));
                add_btn("Copy", Some(CommitContextAction::Copy));
                add_btn("Copy SHA", Some(CommitContextAction::CopySha));
                add_btn("Copy Message", Some(CommitContextAction::CopyMessage));

                // Star/unstar
                bx.append(&gtk::Separator::new(Orientation::Horizontal));
                let item = StarredItem::Commit(oid.clone());
                let already = starred.borrow().contains(&item);
                let label = if already { "Retirer des favoris" } else { "Ajouter aux favoris" };
                let btn_star = gtk::Button::new();
                btn_star.add_css_class("flat");
                btn_star.add_css_class("context-menu-btn");
                btn_star.set_halign(gtk::Align::Fill);
                btn_star.set_hexpand(true);
                let star_lbl = gtk::Label::new(Some(label));
                star_lbl.set_xalign(0.0);
                btn_star.set_child(Some(&star_lbl));
                {
                    let starred_c = starred.clone();
                    let star_label_c2 = star_label_c.clone();
                    let on_star_c2 = on_star_changed.clone();
                    let pop_c = pop.clone();
                    btn_star.connect_clicked(move |_| {
                        let mut st = starred_c.borrow_mut();
                        if already {
                            st.remove(&item);
                            star_label_c2.set_text("☆");
                        } else {
                            st.insert(item.clone());
                            star_label_c2.set_text("★");
                        }
                        if let Some(cb) = &*on_star_c2.borrow() {
                            cb();
                        }
                        pop_c.popdown();
                    });
                }
                bx.append(&btn_star);

                pop.set_child(Some(&bx));
                pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                pop.set_parent(&row_c);
                pop.popup();
            }
        });
        row.add_controller(click);

        row
    }

    pub fn load(&self, commits: Vec<CommitInfo>) {
        {
            let mut all = self.commits.borrow_mut();
            all.clear();
            all.extend(commits.clone());
        }
        self.reload_list(&commits);
    }

    pub fn append(&self, commits: Vec<CommitInfo>) {
        {
            let mut all = self.commits.borrow_mut();
            all.extend(commits.clone());
        }
        let all = self.commits.borrow();
        self.reload_list(&all);
    }

    pub fn connect_on_select<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }

    pub fn filter(&self, query: &str) {
        let all = self.commits.borrow();
        if query.is_empty() {
            self.reload_list(&all);
        } else {
            let q = query.to_lowercase();
            let filtered: Vec<CommitInfo> = all
                .iter()
                .filter(|c| {
                    c.summary.to_lowercase().contains(&q)
                        || c.author.to_lowercase().contains(&q)
                        || c.short_id.contains(&q)
                        || c.oid.contains(&q)
                })
                .cloned()
                .collect();
            self.reload_list(&filtered);
        }
    }

    fn reload_list(&self, commits: &[CommitInfo]) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let graphs = Self::compute_graph(commits);
        for (c, g) in commits.iter().zip(graphs.iter()) {
            let row = Self::row_from_commit(
                c,
                g,
                self.starred.clone(),
                self.on_star_changed.clone(),
                self.on_action.clone(),
            );
            self.list.append(&row);
        }
    }

    fn compute_graph(commits: &[CommitInfo]) -> Vec<GraphRowData> {
        let mut lanes: Vec<Option<String>> = Vec::new();
        let mut rows = Vec::new();

        for c in commits {
            let active_before = lanes.clone();

            let node_lane = if let Some(idx) = lanes.iter().position(|o| o.as_ref() == Some(&c.oid))
            {
                idx
            } else {
                let idx = lanes.iter().position(|o| o.is_none()).unwrap_or_else(|| {
                    lanes.push(None);
                    lanes.len() - 1
                });
                idx
            };

            if node_lane >= lanes.len() {
                lanes.resize(node_lane + 1, None);
            }

            let mut parent_lanes = Vec::new();
            for (i, p) in c.parents.iter().enumerate() {
                if i == 0 {
                    parent_lanes.push(node_lane);
                    lanes[node_lane] = Some(p.clone());
                } else {
                    let idx = if let Some(pos) = lanes.iter().position(|o| o.as_ref() == Some(p)) {
                        pos
                    } else {
                        let pos = lanes.iter().position(|o| o.is_none()).unwrap_or_else(|| {
                            lanes.push(None);
                            lanes.len() - 1
                        });
                        pos
                    };
                    parent_lanes.push(idx);
                    if idx >= lanes.len() {
                        lanes.resize(idx + 1, None);
                    }
                    lanes[idx] = Some(p.clone());
                }
            }

            // Any lane that carried this commit before this row should connect
            // into the node in the top half of the row (branching point visual).
            let incoming_lanes: Vec<usize> = active_before
                .iter()
                .enumerate()
                .filter_map(|(i, o)| if o.as_ref() == Some(&c.oid) && i != node_lane { Some(i) } else { None })
                .collect();

            for l in lanes.iter_mut() {
                if l.as_ref() == Some(&c.oid) {
                    *l = None;
                }
            }

            let lane_count = lanes.len();
            rows.push(GraphRowData {
                active_before,
                node_lane,
                parent_lanes,
                lane_count,
                incoming_lanes,
            });
        }

        rows
    }
}
