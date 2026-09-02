use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::{domain::JobStatus, theme};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Screen {
    #[default]
    Source,
    Probe,
    Options,
    Review,
    Progress,
    Done,
}

impl Screen {
    fn step_index(self) -> usize {
        match self {
            Self::Source | Self::Probe => 0,
            Self::Options => 1,
            Self::Review => 2,
            Self::Progress => 3,
            Self::Done => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Overlay {
    History,
    Settings,
    Help,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoverTarget {
    SourceField(usize),
    OptionsField(usize),
    SourceContinue,
    OptionsReview,
    OptionsBack,
    CookieEnable,
    CookieDisable,
    ReviewStart,
    ReviewBack,
    ProgressCancel,
    DoneNew,
    DoneRetry,
    DoneOpen,
    ProbeCancel,
    HistoryClear,
    OverlayClose,
    SettingRow(usize),
    SettingsEdit,
    SettingsSave,
    SettingsCancel,
    Help,
    History,
    Settings,
    Quit,
}

fn cookie_actions() -> Vec<(&'static str, HoverTarget)> {
    vec![
        ("Enter Enable cookies", HoverTarget::CookieEnable),
        ("Esc Disable", HoverTarget::CookieDisable),
    ]
}

fn navigation_actions() -> Vec<(&'static str, HoverTarget)> {
    vec![
        ("F1 Help", HoverTarget::Help),
        ("F2 History", HoverTarget::History),
        ("F3 Settings", HoverTarget::Settings),
        ("q Quit", HoverTarget::Quit),
    ]
}

fn overlay_actions(model: &UiModel) -> Vec<(&'static str, HoverTarget)> {
    match model.overlay {
        Some(Overlay::History) => vec![
            ("x Clear history", HoverTarget::HistoryClear),
            ("Esc Close", HoverTarget::OverlayClose),
        ],
        Some(Overlay::Settings) if model.settings_editing => vec![
            ("Enter Save", HoverTarget::SettingsSave),
            ("Esc Cancel edit", HoverTarget::SettingsCancel),
        ],
        Some(Overlay::Settings) => vec![
            ("Enter Edit", HoverTarget::SettingsEdit),
            ("s Save", HoverTarget::SettingsSave),
            ("Esc Close", HoverTarget::OverlayClose),
        ],
        Some(Overlay::Help) => vec![("Esc Close", HoverTarget::OverlayClose)],
        None => Vec::new(),
    }
}

fn screen_actions(model: &UiModel) -> Vec<(&'static str, HoverTarget)> {
    match model.screen {
        Screen::Source => vec![("Enter Continue", HoverTarget::SourceContinue)],
        Screen::Probe => vec![("Esc Stop reading", HoverTarget::ProbeCancel)],
        Screen::Options => vec![
            ("Enter Review", HoverTarget::OptionsReview),
            ("Esc Back", HoverTarget::OptionsBack),
        ],
        Screen::Review => vec![
            ("Enter Start download", HoverTarget::ReviewStart),
            ("Esc Back", HoverTarget::ReviewBack),
        ],
        Screen::Progress => vec![("c Cancel", HoverTarget::ProgressCancel)],
        Screen::Done => {
            let mut actions = vec![("Enter New download", HoverTarget::DoneNew)];
            if model
                .current_job
                .as_ref()
                .is_some_and(|job| job.status.is_retryable())
            {
                actions.push(("r Retry", HoverTarget::DoneRetry));
            }
            actions.push(("o Open output", HoverTarget::DoneOpen));
            actions
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencySummary {
    pub yt_dlp: String,
    pub ffmpeg: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JobDetails {
    pub title: String,
    pub source: String,
    pub format: String,
    pub output: String,
    pub status: JobStatus,
    pub progress_percent: u16,
    pub downloaded: String,
    pub total: String,
    pub speed: String,
    pub eta: String,
    pub log_lines: Vec<String>,
    pub log_offset: u16,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkflowView {
    pub focused_field: usize,
    pub source: String,
    pub authentication: String,
    pub profile: String,
    pub probe_summary: Vec<String>,
    pub mode: String,
    pub quality: String,
    pub subtitle: String,
    pub output: String,
    pub review_lines: Vec<String>,
    pub review_scroll: u16,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HistoryRow {
    pub title: String,
    pub result: String,
    pub finished_at: String,
    pub output: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SettingField {
    pub name: String,
    pub value: String,
    pub hint: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiModel {
    pub screen: Screen,
    pub overlay: Option<Overlay>,
    pub dependencies: DependencySummary,
    pub current_job: Option<JobDetails>,
    pub workflow: WorkflowView,
    pub history_rows: Vec<HistoryRow>,
    pub settings_fields: Vec<SettingField>,
    pub selected_setting: usize,
    pub settings_editing: bool,
    pub cookie_notice_pending: bool,
    pub hover_target: Option<HoverTarget>,
    pub status_message: Option<String>,
}

pub fn draw(frame: &mut Frame, model: &UiModel) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::BACKGROUND)),
        area,
    );

    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Length(4),
    ])
    .split(area);

    draw_header(frame, layout[0], model);
    draw_steps(frame, layout[1], model.screen);
    let card = card_rect(area, model);
    match model.overlay {
        Some(Overlay::History) => draw_history(frame, card, model),
        Some(Overlay::Settings) => draw_settings(frame, card, model),
        Some(Overlay::Help) => draw_help(frame, card),
        None => draw_screen(frame, card, model),
    }
    draw_footer(frame, layout[3], model);
}

fn card_rect(area: Rect, model: &UiModel) -> Rect {
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Length(4),
    ])
    .split(area);
    compact_card(centered_card(layout[2]), model)
}

#[cfg(test)]
pub(crate) fn card_rect_for_test(area: Rect, model: &UiModel) -> Rect {
    card_rect(area, model)
}

pub fn hit_test(area: Rect, model: &UiModel, column: u16, row: u16) -> Option<HoverTarget> {
    let card = card_rect(area, model);
    let inner_x = card.x.saturating_add(1);
    let inner_width = card.width.saturating_sub(2);
    let settings_row_target = |index: usize, target| {
        let rect = Rect::new(
            inner_x,
            card.y.saturating_add(1 + index as u16 * 2),
            inner_width,
            2,
        );
        rect.contains((column, row).into()).then_some(target)
    };

    if model.cookie_notice_pending {
        return footer_hit(area, 1, cookie_actions(), column, row);
    }
    if let Some(overlay) = model.overlay {
        match overlay {
            Overlay::Settings => {
                for index in 0..model.settings_fields.len() {
                    if let Some(target) = settings_row_target(index, HoverTarget::SettingRow(index))
                    {
                        return Some(target);
                    }
                }
            }
            Overlay::History | Overlay::Help => {}
        }
        return footer_hit(area, 1, overlay_actions(model), column, row)
            .or_else(|| footer_hit(area, 2, navigation_actions(), column, row));
    }

    match model.screen {
        Screen::Source => {
            let fields = [
                ("URL", &model.workflow.source),
                ("Cookies", &model.workflow.authentication),
                ("Profile", &model.workflow.profile),
            ];
            for (index, (name, value)) in fields.into_iter().enumerate() {
                let width = field_line_width(name, value).min(inner_width);
                let rect = Rect::new(inner_x, card.y + 1 + index as u16, width, 1);
                if rect.contains((column, row).into()) {
                    let target = HoverTarget::SourceField(index);
                    return Some(target);
                }
            }
        }
        Screen::Options => {
            let fields = [
                ("Mode", &model.workflow.mode, true),
                (
                    "Quality",
                    &model.workflow.quality,
                    model.workflow.quality != "Not used",
                ),
                (
                    "Subtitle",
                    &model.workflow.subtitle,
                    model.workflow.subtitle != "Not used",
                ),
                ("Output", &model.workflow.output, true),
            ];
            for (index, (name, value, interactive)) in fields.into_iter().enumerate() {
                if !interactive {
                    continue;
                }
                let width = field_line_width(name, value).min(inner_width);
                let rect = Rect::new(inner_x, card.y + 1 + index as u16, width, 1);
                if rect.contains((column, row).into()) {
                    let target = HoverTarget::OptionsField(index);
                    return Some(target);
                }
            }
        }
        _ => {}
    }
    footer_hit(area, 1, screen_actions(model), column, row)
        .or_else(|| footer_hit(area, 2, navigation_actions(), column, row))
}

fn footer_hit(
    area: Rect,
    line: u16,
    segments: Vec<(&'static str, HoverTarget)>,
    column: u16,
    row: u16,
) -> Option<HoverTarget> {
    let total = segments.iter().map(|(label, _)| label.len()).sum::<usize>()
        + segments.len().saturating_sub(1) * 3;
    let mut x = area.x + area.width / 2 - total as u16 / 2;
    let y = area.y + area.height.saturating_sub(3) + line;
    for (label, target) in segments {
        let rect = Rect::new(x, y, label.len() as u16, 1);
        if rect.contains((column, row).into()) {
            return Some(target);
        }
        x = x.saturating_add(label.len() as u16 + 3);
    }
    None
}

fn centered_card(area: Rect) -> Rect {
    if area.width <= 104 {
        return area;
    }
    Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(100),
        Constraint::Fill(1),
    ])
    .split(area)[1]
}

fn compact_card(area: Rect, model: &UiModel) -> Rect {
    let requested_height = match model.overlay {
        Some(Overlay::History) => 18,
        Some(Overlay::Settings) => model
            .settings_fields
            .len()
            .saturating_mul(2)
            .saturating_add(2)
            .min(18) as u16,
        Some(Overlay::Help) => 8,
        None => match model.screen {
            Screen::Source => 5,
            Screen::Probe => model.workflow.probe_summary.len().saturating_add(6).min(14) as u16,
            Screen::Options => 6,
            Screen::Review => model.workflow.review_lines.len().saturating_add(9).min(18) as u16,
            Screen::Progress => area.height,
            Screen::Done => match model
                .current_job
                .as_ref()
                .and_then(|job| job.error.as_ref())
            {
                Some(_) => 10,
                None => 7,
            },
        },
    };
    let requested_height = requested_height.min(area.height);
    Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(requested_height),
        Constraint::Fill(1),
    ])
    .split(area)[1]
}

fn draw_header(frame: &mut Frame, area: Rect, model: &UiModel) {
    let line = Line::from(vec![
        Span::styled(
            " FetchDeck ",
            Style::default()
                .fg(theme::HEADING)
                .bg(theme::PANEL)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  yt-dlp {}  ffmpeg {} ",
                model.dependencies.yt_dlp, model.dependencies.ffmpeg
            ),
            theme::faint().bg(theme::PANEL),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme::BORDER)),
            )
            .style(Style::default().bg(theme::PANEL)),
        area,
    );
}

fn draw_steps(frame: &mut Frame, area: Rect, screen: Screen) {
    let current = screen.step_index();
    let names = ["Source", "Options", "Review", "Progress", "Done"];
    let column_width = area.width / names.len() as u16;
    let steps_width = column_width.saturating_mul(names.len() as u16);
    let steps_x = area.x + area.width.saturating_sub(steps_width) / 2;
    let marker_y = area.y;
    let label_y = area.y.saturating_add(1);

    for index in 1..names.len() {
        let previous_center = steps_x + column_width * (index as u16 - 1) + column_width / 2;
        let center = steps_x + column_width * index as u16 + column_width / 2;
        let connector = Rect::new(
            previous_center.saturating_add(1),
            marker_y,
            center.saturating_sub(previous_center).saturating_sub(1),
            1,
        );
        frame.render_widget(
            Paragraph::new("─".repeat(connector.width as usize)).style(theme::faint()),
            connector,
        );
    }

    for (index, name) in names.iter().enumerate() {
        let style = if index < current {
            Style::default().fg(theme::SUCCESS)
        } else if index == current {
            Style::default()
                .fg(theme::FOCUS)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::faint()
        };
        let center = steps_x + column_width * index as u16 + column_width / 2;
        frame.render_widget(
            Paragraph::new(Span::styled(if index < current { "●" } else { "○" }, style)),
            Rect::new(center, marker_y, 1, 1),
        );
        frame.render_widget(
            Paragraph::new(Span::styled(*name, style)),
            Rect::new(
                center.saturating_sub(name.len() as u16 / 2),
                label_y,
                name.len() as u16,
                1,
            ),
        );
    }
}

fn draw_screen(frame: &mut Frame, area: Rect, model: &UiModel) {
    match model.screen {
        Screen::Source | Screen::Probe => draw_source(frame, area, model),
        Screen::Options => draw_options(frame, area, &model.workflow, model.hover_target),
        Screen::Review => draw_review(frame, area, &model.workflow),
        Screen::Progress => draw_progress(frame, area, model.current_job.as_ref()),
        Screen::Done => draw_done(frame, area, model.current_job.as_ref()),
    }
}

fn draw_source(frame: &mut Frame, area: Rect, model: &UiModel) {
    let source = &model.workflow;
    let mut lines = vec![
        field_line_with_focus(
            "URL",
            &source.source,
            source.focused_field == 0,
            model.hover_target == Some(HoverTarget::SourceField(0)),
        ),
        field_line_with_focus(
            "Cookies",
            &source.authentication,
            source.focused_field == 1,
            model.hover_target == Some(HoverTarget::SourceField(1)),
        ),
        field_line_with_focus(
            "Profile",
            &source.profile,
            source.focused_field == 2,
            model.hover_target == Some(HoverTarget::SourceField(2)),
        ),
    ];
    if model.screen == Screen::Probe {
        lines.push(Line::from(""));
        lines.extend(
            source
                .probe_summary
                .iter()
                .map(|line| Line::from(Span::styled(line, Style::default().fg(theme::WORKING)))),
        );
    }
    frame.render_widget(
        Paragraph::new(lines).block(theme::panel(
            if model.screen == Screen::Probe {
                " Source  Reading metadata "
            } else {
                " Source "
            },
            true,
        )),
        area,
    );
}

fn draw_options(
    frame: &mut Frame,
    area: Rect,
    workflow: &WorkflowView,
    hover: Option<HoverTarget>,
) {
    let lines = vec![
        field_line_with_focus(
            "Mode",
            &workflow.mode,
            workflow.focused_field == 0,
            hover == Some(HoverTarget::OptionsField(0)),
        ),
        field_line_with_focus(
            "Quality",
            &workflow.quality,
            workflow.focused_field == 1,
            hover == Some(HoverTarget::OptionsField(1)),
        ),
        field_line_with_focus(
            "Subtitle",
            &workflow.subtitle,
            workflow.focused_field == 2,
            hover == Some(HoverTarget::OptionsField(2)),
        ),
        field_line_with_focus(
            "Output",
            &workflow.output,
            workflow.focused_field == 3,
            hover == Some(HoverTarget::OptionsField(3)),
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(theme::panel(" Options ", true)),
        area,
    );
}

fn draw_review(frame: &mut Frame, area: Rect, workflow: &WorkflowView) {
    let mut lines = vec![
        field_line("Source", &workflow.source),
        field_line("Cookies", &workflow.authentication),
        field_line("Profile", &workflow.profile),
        field_line("Mode", &workflow.mode),
        field_line("Quality", &workflow.quality),
        field_line("Subtitle", &workflow.subtitle),
        field_line("Output", &workflow.output),
    ];
    lines.extend(
        workflow
            .review_lines
            .iter()
            .map(|line| Line::from(line.as_str())),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((workflow.review_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(theme::panel(" Review ", true)),
        area,
    );
}

fn draw_progress(frame: &mut Frame, area: Rect, job: Option<&JobDetails>) {
    let Some(job) = job else {
        frame.render_widget(
            Paragraph::new("Preparing download...")
                .style(theme::muted())
                .block(theme::panel(" Progress ", true)),
            area,
        );
        return;
    };
    let parts = Layout::vertical([
        Constraint::Length(6.min(area.height.saturating_sub(4))),
        Constraint::Length(1),
        Constraint::Min(2),
    ])
    .split(area);
    let details = vec![
        field_line("Title", &job.title),
        field_line("Format", &job.format),
        field_line("Output", &job.output),
        Line::from(vec![
            Span::styled("Transfer  ", theme::muted()),
            Span::raw(format!(
                "{} / {}  {}  ETA {}",
                job.downloaded, job.total, job.speed, job.eta
            )),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(details)
            .wrap(Wrap { trim: true })
            .block(theme::panel(" Progress ", true)),
        parts[0],
    );
    frame.render_widget(
        Gauge::default()
            .ratio(f64::from(job.progress_percent.min(100)) / 100.0)
            .label(format!(
                "{}  {}%",
                job.status.label(),
                job.progress_percent.min(100)
            ))
            .gauge_style(Style::default().fg(theme::PROGRESS).bg(theme::SURFACE)),
        parts[1],
    );
    let logs = job
        .log_lines
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(logs)
            .scroll((job.log_offset, 0))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme::MUTED).bg(theme::SURFACE))
            .block(theme::panel(" Log ", false)),
        parts[2],
    );
}

fn draw_done(frame: &mut Frame, area: Rect, job: Option<&JobDetails>) {
    let Some(job) = job else {
        frame.render_widget(
            Paragraph::new("No completed download")
                .style(theme::muted())
                .block(theme::panel(" Done ", true)),
            area,
        );
        return;
    };
    let status_style = match job.status {
        JobStatus::Completed => Style::default().fg(theme::SUCCESS),
        JobStatus::Failed | JobStatus::Cancelled => Style::default().fg(theme::ERROR),
        _ => Style::default().fg(theme::FOREGROUND),
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!(
                "{}  {}",
                if job.status == JobStatus::Completed {
                    "✓"
                } else {
                    "×"
                },
                job.status.label()
            ),
            status_style.add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        field_line("Title", &job.title),
        field_line("Format", &job.format),
        field_line("Output", &job.output),
    ];
    if let Some(error) = &job.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(error, status_style)));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(theme::panel(" Done ", true)),
        area,
    );
}

fn draw_history(frame: &mut Frame, area: Rect, model: &UiModel) {
    let rows = model.history_rows.iter().map(|item| {
        Row::new([
            Cell::from(item.title.as_str()),
            Cell::from(item.result.as_str()),
            Cell::from(item.finished_at.as_str()),
            Cell::from(item.output.as_str()),
        ])
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(35),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
                Constraint::Percentage(30),
            ],
        )
        .header(
            Row::new(["TITLE", "RESULT", "FINISHED", "OUTPUT"]).style(
                Style::default()
                    .fg(theme::HEADING)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(theme::panel(" History ", true)),
        area,
    );
}

fn draw_settings(frame: &mut Frame, area: Rect, model: &UiModel) {
    let items = model
        .settings_fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let hovered = model.hover_target == Some(HoverTarget::SettingRow(index));
            let style = if index == model.selected_setting && hovered {
                theme::selected_hovered()
            } else if index == model.selected_setting {
                theme::selected()
            } else if hovered {
                theme::hovered()
            } else {
                Style::default().fg(theme::FOREGROUND).bg(theme::SURFACE)
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{:<18}", field.name), Modifier::BOLD),
                    Span::raw(&field.value),
                ]),
                Line::from(Span::styled(field.hint.as_str(), theme::muted())),
            ])
            .style(style)
        });
    frame.render_widget(
        List::new(items).block(theme::panel(
            if model.settings_editing {
                " Settings  Editing "
            } else {
                " Settings "
            },
            true,
        )),
        area,
    );
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        field_line("Move", "Up / Down or k / j"),
        field_line("Choose", "Left / Right"),
        field_line("Continue", "Enter"),
        field_line("Back", "Esc"),
        field_line("Panels", "F1 Help  F2 History  F3 Settings"),
        field_line("Quit", "q"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(theme::panel(" Help ", true)),
        area,
    );
}

fn draw_footer(frame: &mut Frame, area: Rect, model: &UiModel) {
    let status = model.status_message.as_deref().unwrap_or("Ready");
    let actions = if model.cookie_notice_pending {
        cookie_actions()
    } else {
        match model.overlay {
            Some(_) => overlay_actions(model),
            None => screen_actions(model),
        }
    };
    let action_line = action_line(&actions, model.hover_target);
    let navigation = navigation_actions();
    let lines = vec![
        Line::from(Span::styled(
            format!(" {status} "),
            Style::default().fg(theme::MUTED).bg(theme::PANEL),
        ))
        .alignment(Alignment::Center),
        action_line.alignment(Alignment::Center),
        action_line_with_style(
            &navigation,
            model.hover_target,
            theme::faint().bg(theme::PANEL),
        )
        .alignment(Alignment::Center),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(theme::BORDER)),
            )
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(theme::PANEL)),
        area,
    );
}

fn action_line(
    actions: &[(&'static str, HoverTarget)],
    hovered: Option<HoverTarget>,
) -> Line<'static> {
    action_line_with_style(
        actions,
        hovered,
        Style::default().fg(theme::KEY).bg(theme::PANEL),
    )
}

fn action_line_with_style(
    actions: &[(&'static str, HoverTarget)],
    hovered: Option<HoverTarget>,
    normal: Style,
) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (label, target)) in actions.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("   ", normal));
        }
        spans.push(Span::styled(
            *label,
            if hovered == Some(*target) {
                theme::hovered()
            } else {
                normal
            },
        ));
    }
    Line::from(spans)
}

fn field_line<'a>(name: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{name:<10}"), theme::muted()),
        Span::raw(value),
    ])
}

fn field_line_with_focus<'a>(
    name: &'a str,
    value: &'a str,
    focused: bool,
    hovered: bool,
) -> Line<'a> {
    let marker = if focused { ">" } else { " " };
    let style = if focused && hovered {
        theme::selected_hovered()
    } else if focused {
        theme::selected()
    } else if hovered {
        theme::hovered()
    } else {
        Style::default().fg(theme::FOREGROUND).bg(theme::SURFACE)
    };
    Line::from(format!("{marker} {name:<9}{value}")).style(style)
}

fn field_line_width(name: &str, value: &str) -> u16 {
    field_line_with_focus(name, value, false, false).width() as u16
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;

    fn render(model: &UiModel) -> String {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| draw(frame, model))
            .expect("screen should render");
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn model_for(screen: Screen) -> UiModel {
        UiModel {
            screen,
            dependencies: DependencySummary {
                yt_dlp: "ready".into(),
                ffmpeg: "ready".into(),
            },
            current_job: Some(JobDetails {
                title: "Sample video".into(),
                source: "https://example.com/watch?v=1".into(),
                format: "Video / MP4 / 1080p".into(),
                output: "/Downloads/sample.mp4".into(),
                status: if screen == Screen::Done {
                    JobStatus::Completed
                } else {
                    JobStatus::Downloading
                },
                progress_percent: 42,
                downloaded: "42 MiB".into(),
                total: "68 MiB".into(),
                speed: "4 MiB/s".into(),
                eta: "00:12".into(),
                log_lines: vec!["Downloading media".into()],
                ..JobDetails::default()
            }),
            workflow: WorkflowView {
                focused_field: 0,
                source: "https://example.com/watch?v=1".into(),
                authentication: "Brave cookies".into(),
                profile: "Default".into(),
                mode: "Video / MP4".into(),
                quality: "4K".into(),
                subtitle: "Not used".into(),
                output: "/Downloads".into(),
                review_lines: vec!["Title     Sample video".into()],
                ..WorkflowView::default()
            },
            history_rows: vec![HistoryRow {
                title: "Finished video".into(),
                result: "Completed".into(),
                finished_at: "Today".into(),
                output: "/Downloads/video.mp4".into(),
            }],
            settings_fields: vec![SettingField {
                name: "Output".into(),
                value: "/Downloads".into(),
                hint: "Default download folder".into(),
            }],
            ..UiModel::default()
        }
    }

    #[test]
    fn renders_every_screen_and_overlay_at_minimum_size() {
        for screen in [
            Screen::Source,
            Screen::Probe,
            Screen::Options,
            Screen::Review,
            Screen::Progress,
            Screen::Done,
        ] {
            render(&model_for(screen));
        }
        for overlay in [Overlay::History, Overlay::Settings, Overlay::Help] {
            let mut model = model_for(Screen::Review);
            model.overlay = Some(overlay);
            render(&model);
        }
    }

    #[test]
    fn stepper_uses_the_same_center_for_markers_and_labels() {
        let model = model_for(Screen::Progress);
        for width in [80, 120] {
            let backend = TestBackend::new(width, 24);
            let mut terminal = Terminal::new(backend).expect("terminal should be created");
            terminal
                .draw(|frame| draw(frame, &model))
                .expect("screen should render");
            let buffer = terminal.backend().buffer();
            let marker_y = 3;
            let label_y = 4;
            let label_row = (0..width)
                .map(|x| buffer[(x, label_y)].symbol())
                .collect::<String>();

            for label in ["Source", "Options", "Review", "Progress", "Done"] {
                let label_x = label_row
                    .find(label)
                    .expect("step label should be rendered") as u16;
                let center_x = label_x + label.len() as u16 / 2;
                assert!(
                    matches!(buffer[(center_x, marker_y)].symbol(), "●" | "○"),
                    "{label} marker should be at column {center_x} in a {width}-column terminal"
                );
            }

            assert!(!label_row.contains("Queue"));
            let progress_x = label_row
                .find("Progress")
                .expect("Progress label should be rendered") as u16;
            assert_eq!(buffer[(progress_x, label_y)].fg, theme::FOCUS);
        }
    }

    #[test]
    fn review_has_one_start_action_and_no_queue_language() {
        let output = render(&model_for(Screen::Review));
        assert!(output.contains("Start download"));
        assert!(!output.to_ascii_lowercase().contains("queue"));
        assert!(!output.contains("Downloading media"));
    }

    #[test]
    fn long_log_and_url_stay_inside_the_buffer() {
        let long = "x".repeat(20_000);
        for screen in [Screen::Source, Screen::Progress] {
            let mut model = model_for(screen);
            model.workflow.source = format!("https://example.com/{long}");
            if let Some(job) = model.current_job.as_mut() {
                job.log_lines = vec![long.clone()];
                job.log_offset = u16::MAX;
            }
            let output = render(&model);
            assert_eq!(output.lines().count(), 24);
            assert!(output.lines().all(|line| line.chars().count() <= 80));
        }
    }

    #[test]
    fn long_source_url_does_not_move_cookie_and_profile_rows() {
        let mut model = model_for(Screen::Source);
        model.workflow.source = format!("https://example.com/{}", "x".repeat(200));
        let area = Rect::new(0, 0, 80, 24);
        let card = card_rect(area, &model);
        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &model)).unwrap();
        let buffer = terminal.backend().buffer();
        let cookie_row = (card.x + 1..card.x + card.width - 1)
            .map(|x| buffer[(x, card.y + 2)].symbol())
            .collect::<String>();
        let profile_row = (card.x + 1..card.x + card.width - 1)
            .map(|x| buffer[(x, card.y + 3)].symbol())
            .collect::<String>();

        assert!(cookie_row.contains("Cookies"));
        assert!(profile_row.contains("Profile"));
        assert_eq!(
            hit_test(area, &model, card.x + 1, card.y + 2),
            Some(HoverTarget::SourceField(1))
        );
        assert_eq!(
            hit_test(area, &model, card.x + 1, card.y + 3),
            Some(HoverTarget::SourceField(2))
        );
    }

    #[test]
    fn review_content_can_scroll_without_replacing_the_action_row() {
        let mut model = model_for(Screen::Review);
        model.workflow.review_lines = (0..40).map(|index| format!("Detail {index:02}")).collect();
        model.workflow.review_scroll = 36;

        let output = render(&model);
        assert!(output.contains("Detail 39"));
        assert!(output.contains("Start download"));
    }

    #[test]
    fn cookie_notice_replaces_the_normal_source_action() {
        let mut model = model_for(Screen::Source);
        model.cookie_notice_pending = true;

        let output = render(&model);
        assert!(output.contains("Enable cookies"));
        assert!(!output.contains("Enter Continue"));
    }

    #[test]
    fn hit_test_matches_rendered_action_text_and_excludes_spacing() {
        let model = model_for(Screen::Review);
        for width in [79, 80, 81, 120, 121] {
            let area = Rect::new(0, 0, width, 24);
            let backend = TestBackend::new(width, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|frame| draw(frame, &model)).unwrap();
            let buffer = terminal.backend().buffer();
            let action_row = (0..width)
                .map(|x| buffer[(x, 22)].symbol())
                .collect::<String>();
            let start = action_row.find("Enter Start download").unwrap() as u16;
            let back = action_row.find("Esc Back").unwrap() as u16;

            assert_eq!(
                hit_test(area, &model, start, 22),
                Some(HoverTarget::ReviewStart),
                "start action should match at width {width}"
            );
            assert_eq!(
                hit_test(area, &model, back, 22),
                Some(HoverTarget::ReviewBack),
                "back action should match at width {width}"
            );
            assert_eq!(
                hit_test(area, &model, start.saturating_sub(1), 22),
                None,
                "spacing should not be clickable at width {width}"
            );
        }
    }

    #[test]
    fn hover_style_is_distinct_from_keyboard_focus() {
        let mut model = model_for(Screen::Source);
        model.workflow.focused_field = 0;
        model.hover_target = Some(HoverTarget::SourceField(1));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &model)).unwrap();
        let area = Rect::new(0, 0, 80, 24);
        let card = card_rect(area, &model);
        let buffer = terminal.backend().buffer();

        assert_eq!(
            buffer[(card.x + 1, card.y + 1)].bg,
            theme::SELECTION_BACKGROUND
        );
        assert_eq!(buffer[(card.x + 1, card.y + 2)].bg, theme::HOVER_BACKGROUND);

        model.hover_target = Some(HoverTarget::SourceField(0));
        terminal.draw(|frame| draw(frame, &model)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(card.x + 1, card.y + 1)].bg, theme::HOVER_BACKGROUND);
        assert_eq!(buffer[(card.x + 1, card.y + 1)].fg, theme::FOCUS);
    }

    #[test]
    fn unavailable_option_rows_are_not_clickable() {
        let mut model = model_for(Screen::Options);
        model.workflow.quality = "Not used".into();
        model.workflow.subtitle = "Not used".into();
        let area = Rect::new(0, 0, 80, 24);
        let card = card_rect(area, &model);

        assert_eq!(hit_test(area, &model, card.x + 1, card.y + 2), None);
        assert_eq!(hit_test(area, &model, card.x + 1, card.y + 3), None);
    }

    #[test]
    fn source_hit_area_stops_after_visible_text() {
        let model = model_for(Screen::Source);
        let area = Rect::new(0, 0, 80, 24);
        let card = card_rect(area, &model);
        assert_eq!(
            hit_test(area, &model, card.x + 1, card.y + 1),
            Some(HoverTarget::SourceField(0))
        );
        assert_eq!(
            hit_test(area, &model, card.x + card.width - 2, card.y + 1),
            None
        );
    }

    #[test]
    fn source_card_stays_compact_on_tall_terminals() {
        let backend = TestBackend::new(120, 60);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| draw(frame, &model_for(Screen::Source)))
            .expect("source screen should render");

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(10, 29)].symbol(), "┌");
        assert_eq!(buffer[(10, 33)].symbol(), "└");
    }

    #[test]
    fn done_card_and_fields_are_centered_and_aligned() {
        let backend = TestBackend::new(120, 60);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| draw(frame, &model_for(Screen::Done)))
            .expect("done screen should render");

        let buffer = terminal.backend().buffer();
        let card_top = 28;
        let card_bottom = 34;
        assert_eq!(buffer[(10, card_top)].symbol(), "┌");
        assert_eq!(buffer[(109, card_top)].symbol(), "┐");
        assert_eq!(buffer[(10, card_bottom)].symbol(), "└");
        assert_eq!(buffer[(109, card_bottom)].symbol(), "┘");
        assert_eq!(card_top - 7, 56 - card_bottom - 1);

        let value_starts = [
            "Sample video",
            "Video / MP4 / 1080p",
            "/Downloads/sample.mp4",
        ]
        .into_iter()
        .map(|value| {
            (card_top + 1..card_bottom)
                .find_map(|y| {
                    let row = (0..120)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>();
                    row.find(value).map(|x| x as u16)
                })
                .expect("field value should be rendered")
        })
        .collect::<Vec<_>>();
        assert!(value_starts.windows(2).all(|pair| pair[0] == pair[1]));

        let status_cells = (0..120)
            .filter(|x| buffer[(*x, card_top + 1)].fg == theme::SUCCESS)
            .collect::<Vec<_>>();
        let first_status_cell = *status_cells.first().expect("status should be rendered");
        let last_status_cell = *status_cells.last().expect("status should be rendered");
        let left_gap = first_status_cell - 11;
        let right_gap = 108 - last_status_cell;
        assert!(left_gap.abs_diff(right_gap) <= 1);
    }
}
