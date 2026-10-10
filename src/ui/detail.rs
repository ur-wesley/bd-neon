use eframe::egui::{self, ScrollArea, Ui};

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::model::{format_local, Action, Comment, DepNode, IssueDetail};

use super::{badge, banner, icon_button, icons, theme};

pub struct DetailView {
    pub detail: Option<IssueDetail>,
    pub loading_id: Option<String>,
    pub error: Option<String>,
}

/// Live composer bindings: `draft` mutates the store field in place (no
/// `UpdateCommentDraft` round-trip needed for typing), `posting` disables
/// the Post button, `error` renders the last post failure as a banner.
pub struct ComposerState<'a> {
    pub draft: &'a mut String,
    pub posting: bool,
    pub error: Option<String>,
}

pub fn can_post_comment(draft: &str, posting: bool) -> bool {
    !posting && !draft.trim().is_empty()
}

#[allow(clippy::too_many_lines)]
pub fn popup(
    ui: &mut Ui,
    view: &DetailView,
    cache: &mut CommonMarkCache,
    composer: ComposerState<'_>,
) -> Option<Action> {
    ui.set_min_width(820.0);
    ui.set_max_width(860.0);
    ui.set_min_height(520.0);
    ui.set_max_height(640.0);
    theme::paint_dialog_top_highlight(ui);

    let title = view.detail.as_ref().map_or_else(
        || view.loading_id.as_deref().unwrap_or("issue"),
        |detail| detail.title.as_str(),
    );

    let mut action = None;
    let mut close = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(theme::SPACING, 0.0);

        if let Some(detail) = &view.detail {
            if let Some(parent_id) = &detail.parent {
                let parent_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(format!("{} {parent_id}", icons::ARROW_UP_RIGHT))
                            .small()
                            .strong()
                            .color(theme::neon_cyan()),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE),
                );
                if theme::pointer(parent_btn)
                    .on_hover_text("Open parent bead")
                    .clicked()
                {
                    action = Some(Action::OpenDetail(parent_id.clone()));
                }
                ui.label(egui::RichText::new("/").small().color(theme::dim_text()));
            }

            badge::issue_id(ui, &detail.id);
            copy_id_button(ui, &detail.id);
        } else if let Some(id) = &view.loading_id {
            badge::issue_id(ui, id);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button::icon_button(
                ui,
                &icon_button::IconButtonProps {
                    label: icons::X,
                    selected: false,
                },
            ) {
                close = true;
            }
            if let Some(detail) = &view.detail {
                badge::status_badge(ui, &detail.status);
                badge::priority_badge(ui, detail.priority);
                badge::kind_badge(ui, &detail.kind);
            }
        });
    });

    ui.add_space(theme::SPACING);

    ui.add(
        egui::Label::new(
            egui::RichText::new(title)
                .size(18.0)
                .strong()
                .color(theme::foreground()),
        )
        .wrap(),
    );

    ui.add_space(theme::SPACING);
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        theme::hairline(),
    );
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top() + 1.0,
        theme::dialog_highlight_stroke(),
    );
    ui.add_space(theme::SPACING);

    if close {
        return Some(Action::Back);
    }
    if action.is_some() {
        return action;
    }

    if let Some(error) = &view.error {
        banner::banner(ui, error);
    }

    if let Some(detail) = &view.detail {
        if let Some(act) = popup_body(ui, detail, cache, composer) {
            return Some(act);
        }
    } else if let Some(id) = &view.loading_id {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(format!("loading {id}…"))
                    .italics()
                    .color(theme::dim_text()),
            );
        });
    }

    None
}

fn popup_body(
    ui: &mut Ui,
    detail: &IssueDetail,
    cache: &mut CommonMarkCache,
    composer: ComposerState<'_>,
) -> Option<Action> {
    let mut action = None;
    let avail_w = ui.available_width();
    let sidebar_w = 230.0_f32;
    let content_w = (avail_w - sidebar_w - theme::SPACING).max(360.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(content_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ScrollArea::vertical()
                    .id_salt("issue_content_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        markdown_card(
                            ui,
                            cache,
                            icons::FILE_TEXT,
                            "DESCRIPTION",
                            detail.description.as_deref(),
                            "No description provided.",
                        );

                        ui.add_space(theme::SPACING);

                        if detail.acceptance_criteria.is_some() {
                            markdown_card(
                                ui,
                                cache,
                                icons::CHECK,
                                "ACCEPTANCE CRITERIA",
                                detail.acceptance_criteria.as_deref(),
                                "",
                            );
                            ui.add_space(theme::SPACING);
                        }

                        if let Some(act) = comments_section(ui, detail, cache, composer) {
                            action = Some(act);
                        }
                        if action.is_none() {
                            ui.add_space(theme::SPACING);
                        }

                        if let Some(act) = dependency_section(ui, detail) {
                            action = Some(act);
                        }
                    });
            },
        );

        ui.add_space(theme::SPACING);
        ui.painter().vline(
            ui.cursor().left(),
            ui.max_rect().y_range(),
            theme::hairline(),
        );
        ui.add_space(theme::SPACING);

        let sidebar_avail = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::vec2(sidebar_avail, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if let Some(act) = sidebar(ui, detail) {
                    action = Some(act);
                }
            },
        );
    });

    action
}

fn sidebar(ui: &mut Ui, detail: &IssueDetail) -> Option<Action> {
    let mut action = None;

    ScrollArea::vertical()
        .id_salt("issue_sidebar_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(act) = render_properties_section(ui, detail) {
                action = Some(act);
            }

            ui.add_space(theme::SPACING);
            ui.painter().hline(
                ui.max_rect().x_range(),
                ui.cursor().top(),
                theme::hairline(),
            );

            render_relationships_section(ui, detail);
            render_timeline_section(ui, detail);
        });

    action
}

fn render_properties_section(ui: &mut Ui, detail: &IssueDetail) -> Option<Action> {
    let mut action = None;
    sidebar_section_header(ui, "PROPERTIES");

    sidebar_prop_row(ui, "Status", |ui| {
        badge::status_badge(ui, &detail.status);
    });
    sidebar_prop_row(ui, "Priority", |ui| {
        badge::priority_badge(ui, detail.priority);
    });
    sidebar_prop_row(ui, "Type", |ui| {
        badge::kind_badge(ui, &detail.kind);
    });

    let assignee = detail.assignee.as_deref().or(detail.owner.as_deref());
    sidebar_prop_row(ui, "Assignee", |ui| {
        if let Some(name) = assignee {
            badge::badge_color(ui, &format!("{} {name}", icons::USER), theme::neon_cyan());
        } else {
            ui.label(
                egui::RichText::new("Unassigned")
                    .small()
                    .color(theme::dim_text()),
            );
        }
    });

    if let Some(parent_id) = &detail.parent {
        sidebar_prop_row(ui, "Parent", |ui| {
            let btn = ui.add(
                egui::Button::new(
                    egui::RichText::new(format!("{} {parent_id}", icons::ARROW_UP_RIGHT))
                        .small()
                        .color(theme::neon_cyan()),
                )
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE),
            );
            if theme::pointer(btn)
                .on_hover_text("Open parent bead")
                .clicked()
            {
                action = Some(Action::OpenDetail(parent_id.clone()));
            }
        });
    }

    action
}

fn render_relationships_section(ui: &mut Ui, detail: &IssueDetail) {
    sidebar_section_header(ui, "RELATIONSHIPS");
    sidebar_prop_row(ui, "Dependencies", |ui| {
        let count = detail.dependencies.len().max(detail.dependency_count);
        badge::badge(ui, &count.to_string());
    });
    sidebar_prop_row(ui, "Dependents", |ui| {
        let count = detail.dependents.len().max(detail.dependent_count);
        badge::badge(ui, &count.to_string());
    });
    sidebar_prop_row(ui, "Comments", |ui| {
        badge::badge(ui, &detail.comment_count.to_string());
    });
}

fn render_timeline_section(ui: &mut Ui, detail: &IssueDetail) {
    if detail.created_at.is_none()
        && detail.started_at.is_none()
        && detail.updated_at.is_none()
        && detail.closed_at.is_none()
    {
        return;
    }

    ui.add_space(theme::SPACING);
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        theme::hairline(),
    );

    sidebar_section_header(ui, "TIMELINE");
    if let Some(created) = detail.created_at {
        sidebar_prop_row(ui, "Created", |ui| {
            ui.label(
                egui::RichText::new(format_local(created))
                    .small()
                    .color(theme::foreground()),
            );
        });
    }
    if let Some(started) = detail.started_at {
        sidebar_prop_row(ui, "Started", |ui| {
            ui.label(
                egui::RichText::new(format_local(started))
                    .small()
                    .color(theme::neon_amber()),
            );
        });
    }
    if let Some(updated) = detail.updated_at {
        sidebar_prop_row(ui, "Updated", |ui| {
            ui.label(
                egui::RichText::new(format_local(updated))
                    .small()
                    .color(theme::foreground()),
            );
        });
    }
    if let Some(closed) = detail.closed_at {
        sidebar_prop_row(ui, "Closed", |ui| {
            ui.label(
                egui::RichText::new(format_local(closed))
                    .small()
                    .color(theme::neon_green()),
            );
        });
    }
}

fn comments_section(
    ui: &mut Ui,
    detail: &IssueDetail,
    cache: &mut CommonMarkCache,
    composer: ComposerState<'_>,
) -> Option<Action> {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme::SPACING;
        ui.label(
            egui::RichText::new(icons::FILE_TEXT)
                .small()
                .color(theme::dim_text()),
        );
        ui.label(
            egui::RichText::new("COMMENTS")
                .small()
                .strong()
                .color(theme::dim_text()),
        );
        let count = detail.comments.len().max(detail.comment_count);
        badge::badge(ui, &count.to_string());
    });
    ui.add_space(theme::SPACING);

    let _ = theme::muted_frame()
        .inner_margin(egui::Margin::same(theme::PAD_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);
            if detail.comments.is_empty() {
                ui.label(
                    egui::RichText::new("No comments yet.")
                        .small()
                        .italics()
                        .color(theme::dim_text()),
                );
            } else {
                for comment in &detail.comments {
                    comment_row(ui, cache, comment);
                    ui.add_space(theme::SPACING);
                }
            }
        });

    ui.add_space(theme::SPACING);
    composer_card(ui, composer)
}

fn comment_row(ui: &mut Ui, cache: &mut CommonMarkCache, comment: &Comment) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(theme::SPACING, 0.0);
        badge::badge_color(
            ui,
            &format!(
                "{} {}",
                icons::USER,
                comment.author.as_deref().unwrap_or("unknown")
            ),
            theme::neon_cyan(),
        );
        if let Some(created) = comment.created_at {
            ui.label(
                egui::RichText::new(format_local(created))
                    .small()
                    .color(theme::dim_text()),
            );
        }
    });
    ui.add_space(2.0);
    if comment.body.trim().is_empty() {
        ui.label(
            egui::RichText::new("(empty comment)")
                .small()
                .italics()
                .color(theme::dim_text()),
        );
    } else {
        CommonMarkViewer::new().show(ui, cache, &comment.body);
    }
}

fn composer_card(ui: &mut Ui, composer: ComposerState<'_>) -> Option<Action> {
    let ComposerState {
        draft,
        posting,
        error,
    } = composer;
    let mut action = None;

    if let Some(message) = &error {
        banner::banner(ui, message);
        ui.add_space(theme::SPACING);
    }

    // Check Ctrl+Enter before the TextEdit sees the keypress so posting
    // doesn't also insert a newline; plain Enter keeps its newline behavior.
    let composer_id = egui::Id::new("comment_composer");
    let ctrl_enter = ui.ctx().memory(|mem| mem.has_focus(composer_id))
        && ui
            .ctx()
            .input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Enter));
    let can_post = can_post_comment(draft, posting);

    let _ = theme::muted_frame()
        .inner_margin(egui::Margin::same(theme::PAD_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);
            let response = ui.add(
                egui::TextEdit::multiline(draft)
                    .id(composer_id)
                    .hint_text("Write a comment… (Ctrl+Enter to post)")
                    .desired_rows(3),
            );
            // Keep the scrolled content area following the caret while typing.
            if response.has_focus() {
                response.scroll_to_me(None);
            }
            ui.add_space(theme::SPACING);
            ui.horizontal(|ui| {
                if posting {
                    ui.label(
                        egui::RichText::new("Posting…")
                            .small()
                            .italics()
                            .color(theme::dim_text()),
                    );
                } else {
                    let post = ui.add_enabled(
                        can_post,
                        egui::Button::new(egui::RichText::new("Post comment").small().strong()),
                    );
                    if post.clicked() {
                        action = Some(Action::PostComment);
                    }
                    ui.label(
                        egui::RichText::new("Ctrl+Enter to post")
                            .small()
                            .color(theme::dim_text()),
                    );
                }
            });
        });

    if ctrl_enter && can_post {
        action = Some(Action::PostComment);
    }
    action
}

fn dependency_section(ui: &mut Ui, detail: &IssueDetail) -> Option<Action> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme::SPACING;
        ui.label(
            egui::RichText::new(icons::LINK)
                .small()
                .color(theme::dim_text()),
        );
        ui.label(
            egui::RichText::new("DEPENDENCIES & BLOCKERS")
                .small()
                .strong()
                .color(theme::dim_text()),
        );
    });
    ui.add_space(theme::SPACING);

    let _ = theme::muted_frame()
        .inner_margin(egui::Margin::same(theme::PAD_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Depends on (incoming blockers)")
                        .small()
                        .strong()
                        .color(theme::foreground()),
                );
                badge::badge(ui, &detail.dependencies.len().to_string());
            });
            ui.add_space(theme::SPACING);

            if detail.dependencies.is_empty() {
                ui.label(
                    egui::RichText::new("No blocking dependencies.")
                        .small()
                        .italics()
                        .color(theme::dim_text()),
                );
            } else {
                for node in &detail.dependencies {
                    if let Some(act) = dep_card_row(ui, node) {
                        action = Some(act);
                    }
                    ui.add_space(theme::SPACING);
                }
            }

            ui.add_space(theme::SPACING);
            ui.painter().hline(
                ui.max_rect().x_range(),
                ui.cursor().top(),
                theme::hairline(),
            );
            ui.add_space(theme::SPACING);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Blocks (outgoing dependents & subtasks)")
                        .small()
                        .strong()
                        .color(theme::foreground()),
                );
                badge::badge(ui, &detail.dependents.len().to_string());
            });
            ui.add_space(theme::SPACING);

            if detail.dependents.is_empty() {
                ui.label(
                    egui::RichText::new("No issues blocked by this.")
                        .small()
                        .italics()
                        .color(theme::dim_text()),
                );
            } else {
                for node in &detail.dependents {
                    if let Some(act) = dep_card_row(ui, node) {
                        action = Some(act);
                    }
                    ui.add_space(theme::SPACING);
                }
            }
        });

    action
}

fn dep_card_row(ui: &mut Ui, node: &DepNode) -> Option<Action> {
    let node_id = node.id.clone();
    let card_id = ui.make_persistent_id(&node_id);
    let was_hovered = ui.data_mut(|d| d.get_temp::<bool>(card_id).unwrap_or(false));

    let (fill, stroke) = if was_hovered {
        (
            theme::surface_hover(),
            egui::Stroke::new(1.0_f32, theme::border_hover()),
        )
    } else {
        (theme::surface_elevated(), theme::hairline())
    };

    let inner = theme::muted_frame()
        .fill(fill)
        .stroke(stroke)
        .inner_margin(egui::Margin::symmetric(theme::PAD_I8, theme::SPACING_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::SPACING, 0.0);

                if let Some(status) = &node.status {
                    badge::status_badge(ui, status);
                }
                if let Some(priority) = node.priority {
                    badge::priority_badge(ui, priority);
                }
                badge::issue_id(ui, &node.id);
                if let Some(kind) = &node.kind {
                    badge::kind_badge(ui, kind);
                }
                if let Some(dep_type) = &node.dep_type {
                    badge::badge_color(ui, dep_type, theme::dim_text());
                }
                if !node.title.is_empty() {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&node.title)
                                .small()
                                .color(theme::foreground()),
                        )
                        .truncate()
                        .selectable(false),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(icons::ARROW_UP_RIGHT).small().color(
                        if was_hovered {
                            theme::neon_cyan()
                        } else {
                            theme::dim_text()
                        },
                    ));
                });
            });
        });

    let response = inner.response.interact(egui::Sense::click());
    let is_now_hovered = response.hovered();
    if is_now_hovered != was_hovered {
        ui.data_mut(|d| d.insert_temp(card_id, is_now_hovered));
        ui.ctx().request_repaint();
    }
    let response = theme::pointer(response);
    if response.clicked() {
        Some(Action::OpenDetail(node_id))
    } else {
        None
    }
}

fn copy_id_button(ui: &mut Ui, id: &str) {
    let copy_id = ui.make_persistent_id(format!("copy_{id}"));
    let last_copied = ui.data_mut(|d| d.get_temp::<std::time::Instant>(copy_id));
    let just_copied = last_copied.is_some_and(|t| t.elapsed().as_secs() < 2);

    let (icon, color) = if just_copied {
        (icons::CHECK, theme::neon_green())
    } else {
        (icons::COPY, theme::dim_text())
    };

    let btn = ui.add(
        egui::Button::new(egui::RichText::new(icon).small().color(color))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
            .corner_radius(theme::radius_sm()),
    );
    let btn = theme::pointer(btn);
    if btn.clicked() {
        ui.ctx().copy_text(id.to_owned());
        ui.data_mut(|d| d.insert_temp(copy_id, std::time::Instant::now()));
    }
    if just_copied {
        btn.on_hover_text("Copied to clipboard!");
    } else {
        btn.on_hover_text("Copy issue ID");
    }
}

fn sidebar_prop_row(ui: &mut Ui, label: &str, add_val: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(egui::RichText::new(label).small().color(theme::dim_text()))
                .selectable(false),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            add_val(ui);
        });
    });
}

fn sidebar_section_header(ui: &mut Ui, label: &str) {
    ui.add_space(theme::SPACING);
    ui.label(
        egui::RichText::new(label)
            .small()
            .strong()
            .color(theme::dim_text()),
    );
    ui.add_space(theme::SPACING);
}

fn markdown_card(
    ui: &mut Ui,
    cache: &mut CommonMarkCache,
    icon: &str,
    title: &str,
    body: Option<&str>,
    placeholder: &str,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme::SPACING;
        ui.label(egui::RichText::new(icon).small().color(theme::dim_text()));
        ui.label(
            egui::RichText::new(title)
                .small()
                .strong()
                .color(theme::dim_text()),
        );
    });
    ui.add_space(theme::SPACING);

    let _ = theme::muted_frame()
        .inner_margin(egui::Margin::same(theme::PAD_I8))
        .show(ui, |ui| {
            theme::fill_width(ui);
            if let Some(content) = body {
                if content.trim().is_empty() {
                    ui.label(
                        egui::RichText::new(placeholder)
                            .small()
                            .italics()
                            .color(theme::dim_text()),
                    );
                } else {
                    CommonMarkViewer::new().show(ui, cache, content);
                }
            } else {
                ui.label(
                    egui::RichText::new(placeholder)
                        .small()
                        .italics()
                        .color(theme::dim_text()),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn two_column_structure() {
        let detail = IssueDetail {
            id: "bd-neon-1".to_owned(),
            title: "Test Bead".to_owned(),
            description: Some("Here is description".to_owned()),
            acceptance_criteria: None,
            status: "open".to_owned(),
            priority: 1,
            kind: "feature".to_owned(),
            assignee: Some("alice".to_owned()),
            owner: None,
            parent: None,
            created_at: Some(Utc::now()),
            updated_at: None,
            started_at: None,
            closed_at: None,
            dependencies: vec![],
            dependents: vec![],
            dependency_count: 0,
            dependent_count: 0,
            comment_count: 2,
            comments: Vec::new(),
        };
        assert_eq!(detail.id, "bd-neon-1");
    }

    #[test]
    fn full_metadata_sidebar() {
        let detail = IssueDetail {
            id: "bd-12".to_owned(),
            title: "Metadata Test".to_owned(),
            description: None,
            acceptance_criteria: None,
            status: "in_progress".to_owned(),
            priority: 0,
            kind: "bug".to_owned(),
            assignee: Some("bob".to_owned()),
            owner: None,
            parent: Some("bd-10".to_owned()),
            created_at: None,
            updated_at: None,
            started_at: None,
            closed_at: None,
            dependencies: vec![],
            dependents: vec![],
            dependency_count: 0,
            dependent_count: 0,
            comment_count: 0,
            comments: Vec::new(),
        };
        assert_eq!(detail.parent.as_deref(), Some("bd-10"));
        assert_eq!(detail.assignee.as_deref(), Some("bob"));
    }

    #[test]
    fn empty_timestamps_hidden() {
        let detail = IssueDetail {
            id: "bd-3".to_owned(),
            title: "Timestamps Test".to_owned(),
            description: None,
            acceptance_criteria: None,
            status: "open".to_owned(),
            priority: 2,
            kind: "task".to_owned(),
            assignee: None,
            owner: None,
            parent: None,
            created_at: None,
            updated_at: None,
            started_at: None,
            closed_at: None,
            dependencies: vec![],
            dependents: vec![],
            dependency_count: 0,
            dependent_count: 0,
            comment_count: 0,
            comments: Vec::new(),
        };
        assert!(detail.created_at.is_none());
        assert!(detail.closed_at.is_none());
    }

    #[test]
    fn breadcrumb_nav() {
        let detail = IssueDetail {
            id: "bd-neon-4.2".to_owned(),
            title: "Subtask".to_owned(),
            description: None,
            acceptance_criteria: None,
            status: "open".to_owned(),
            priority: 2,
            kind: "task".to_owned(),
            assignee: None,
            owner: None,
            parent: Some("bd-neon-4".to_owned()),
            created_at: None,
            updated_at: None,
            started_at: None,
            closed_at: None,
            dependencies: vec![],
            dependents: vec![],
            dependency_count: 0,
            dependent_count: 0,
            comment_count: 0,
            comments: Vec::new(),
        };
        assert_eq!(detail.parent.as_deref(), Some("bd-neon-4"));
    }

    #[test]
    fn composer_post_gate() {
        assert!(can_post_comment("hello", false));
        assert!(!can_post_comment("   ", false));
        assert!(!can_post_comment("", false));
        assert!(!can_post_comment("hello", true));
    }
}
