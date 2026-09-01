use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
};

use crate::theme;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Page {
    #[default]
    Queue,
    AddJob,
    History,
    Settings,
    Help,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AddStage {
    #[default]
    Source,
    Probe,
    Options,
    Review,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencySummary {
    pub yt_dlp: String,
    pub ffmpeg: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueueRow {
    pub title: String,
    pub status: String,
    pub progress_percent: u16,
    pub speed: String,
    pub eta: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JobDetails {
    pub title: String,
    pub source: String,
    pub format: String,
    pub output: String,
    pub status: String,
    pub progress_percent: u16,
    pub downloaded: String,
    pub total: String,
    pub speed: String,
    pub eta: String,
    pub transport_stage: usize,
    pub log_lines: Vec<String>,
    pub log_offset: u16,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AddJobView {
    pub stage: AddStage,
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
    pub active_page: Page,
    pub dependencies: DependencySummary,
    pub queue_rows: Vec<QueueRow>,
    pub selected_queue: usize,
    pub selected_job: Option<JobDetails>,
    pub add_job: AddJobView,
    pub history_rows: Vec<HistoryRow>,
    pub settings_fields: Vec<SettingField>,
    pub selected_setting: usize,
    pub settings_editing: bool,
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
        Constraint::Min(8),
        Constraint::Length(4),
    ])
    .split(area);

    draw_header(frame, layout[0], model);
    match model.active_page {
        Page::Queue => draw_queue(frame, layout[1], model),
        Page::AddJob => draw_add_job(frame, layout[1], &model.add_job),
        Page::History => draw_history(frame, layout[1], model),
        Page::Settings => draw_settings(frame, layout[1], model),
        Page::Help => draw_help(frame, layout[1]),
    }
    draw_footer(frame, layout[2], model);
}

fn draw_header(frame: &mut Frame, area: Rect, model: &UiModel) {
    let page = match model.active_page {
        Page::Queue => "QUEUE",
        Page::AddJob => "ADD JOB",
        Page::History => "HISTORY",
        Page::Settings => "SETTINGS",
        Page::Help => "HELP",
    };
    let line = Line::from(vec![
        Span::styled(
            " YT-DLP TUI ",
            Style::default()
                .fg(theme::HEADING)
                .bg(theme::PANEL)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {page}"),
            Style::default().fg(theme::FOREGROUND).bg(theme::PANEL),
        ),
        Span::styled(
            format!(
                "    yt-dlp {}  ffmpeg {} ",
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

fn draw_queue(frame: &mut Frame, area: Rect, model: &UiModel) {
    if area.width >= 96 {
        let columns = Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area);
        draw_queue_list(frame, columns[0], model);
        draw_job_detail(frame, columns[1], model.selected_job.as_ref());
    } else {
        let rows =
            Layout::vertical([Constraint::Percentage(35), Constraint::Percentage(65)]).split(area);
        draw_queue_list(frame, rows[0], model);
        draw_job_detail(frame, rows[1], model.selected_job.as_ref());
    }
}

fn draw_queue_list(frame: &mut Frame, area: Rect, model: &UiModel) {
    let visible_rows = area.height.saturating_sub(3).max(1) as usize;
    let start = model
        .selected_queue
        .saturating_sub(visible_rows.saturating_sub(1));
    let rows = model
        .queue_rows
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
        .map(|(index, job)| {
            let style = if index == model.selected_queue {
                theme::selected()
            } else {
                Style::default().fg(theme::FOREGROUND).bg(theme::SURFACE)
            };
            Row::new(vec![
                Cell::from(job.title.as_str()),
                Cell::from(job.status.as_str()),
                Cell::from(format!("{}%", job.progress_percent.min(100))),
            ])
            .style(style)
        });
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(58),
            Constraint::Percentage(25),
            Constraint::Percentage(17),
        ],
    )
    .header(
        Row::new(["TITLE", "STATE", "GET"]).style(
            Style::default()
                .fg(theme::HEADING)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(theme::panel(" Queue ", true))
    .column_spacing(1);
    frame.render_widget(table, area);
}

fn draw_job_detail(frame: &mut Frame, area: Rect, job: Option<&JobDetails>) {
    let Some(job) = job else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("No job selected"),
                Line::from(""),
                Line::from(Span::styled(
                    "SOURCE -> PROBE -> QUEUE -> GET -> MERGE -> DONE",
                    theme::faint(),
                )),
                Line::from("Press a to add a video URL."),
            ])
            .style(theme::muted())
            .block(theme::panel(" Transport ", false)),
            area,
        );
        return;
    };

    let parts = Layout::vertical([
        Constraint::Length(7.min(area.height.saturating_sub(4))),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(2),
    ])
    .split(area);
    let progress = job.progress_percent.min(100);
    let detail = vec![
        Line::from(vec![
            Span::styled("Title  ", theme::muted()),
            Span::raw(&job.title),
        ]),
        Line::from(vec![
            Span::styled("Source ", theme::muted()),
            Span::raw(&job.source),
        ]),
        Line::from(vec![
            Span::styled("Format ", theme::muted()),
            Span::raw(&job.format),
        ]),
        Line::from(vec![
            Span::styled("Output ", theme::muted()),
            Span::raw(&job.output),
        ]),
        Line::from(vec![
            Span::styled("Transfer ", theme::muted()),
            Span::raw(format!(
                "{} / {}  {}  ETA {}",
                job.downloaded, job.total, job.speed, job.eta
            )),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(detail)
            .wrap(Wrap { trim: true })
            .block(theme::panel(" Transport ", false)),
        parts[0],
    );

    frame.render_widget(transport_rail(job.transport_stage), parts[1]);
    frame.render_widget(
        Gauge::default()
            .ratio(f64::from(progress) / 100.0)
            .label(format!("{}  {progress}%", job.status))
            .gauge_style(Style::default().fg(theme::PROGRESS).bg(theme::SURFACE)),
        parts[2],
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
            .block(theme::panel(" Log  Up/Down Scroll ", false)),
        parts[3],
    );
}

fn draw_add_job(frame: &mut Frame, area: Rect, add: &AddJobView) {
    let parts = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(2),
    ])
    .split(area);
    draw_add_rail(frame, parts[0], add.stage);

    let (title, lines) = match add.stage {
        AddStage::Source => (
            " Source ",
            vec![
                field_line_with_focus("URL", &add.source, add.focused_field == 0),
                field_line_with_focus("Cookies", &add.authentication, add.focused_field == 1),
                field_line_with_focus("Profile", &add.profile, add.focused_field == 2),
            ],
        ),
        AddStage::Probe => (
            " Probe ",
            add.probe_summary
                .iter()
                .map(|line| Line::from(line.as_str()))
                .collect(),
        ),
        AddStage::Options => (
            " Options ",
            vec![
                field_line_with_focus("Mode", &add.mode, add.focused_field == 0),
                field_line_with_focus("Quality", &add.quality, add.focused_field == 1),
                field_line_with_focus("Subtitle", &add.subtitle, add.focused_field == 2),
                field_line_with_focus("Output", &add.output, add.focused_field == 3),
            ],
        ),
        AddStage::Review => {
            let mut review = vec![
                field_line("Source", &add.source),
                field_line("Cookies", &add.authentication),
                field_line("Profile", &add.profile),
                field_line("Mode", &add.mode),
                field_line("Quality", &add.quality),
                field_line("Subtitle", &add.subtitle),
                field_line("Output", &add.output),
            ];
            review.extend(
                add.review_lines
                    .iter()
                    .map(|line| Line::from(line.as_str())),
            );
            (" Review ", review)
        }
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(theme::panel(title, true)),
        parts[1],
    );
    let actions = match add.stage {
        AddStage::Source => "Tab Field  Left/Right Choose  Enter Probe  Esc Cancel",
        AddStage::Probe => "Esc Back",
        AddStage::Options => "Tab Field  Left/Right Choose  Enter Review  Esc Back",
        AddStage::Review => "Enter Add to queue  Esc Back",
    };
    frame.render_widget(
        Paragraph::new(actions)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme::KEY)),
        parts[2],
    );
}

fn draw_add_rail(frame: &mut Frame, area: Rect, stage: AddStage) {
    let current = match stage {
        AddStage::Source => 0,
        AddStage::Probe => 1,
        AddStage::Options => 2,
        AddStage::Review => 3,
    };
    let names = ["SOURCE", "PROBE", "OPTIONS", "REVIEW"];
    let mut spans = Vec::new();
    for (index, name) in names.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" -> ", theme::faint()));
        }
        let style = if index < current {
            Style::default().fg(theme::SUCCESS)
        } else if index == current {
            Style::default()
                .fg(theme::FOCUS)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::faint()
        };
        spans.push(Span::styled(format!(" {name} "), style));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .block(theme::panel(" Add Job ", false)),
        area,
    );
}

fn draw_history(frame: &mut Frame, area: Rect, model: &UiModel) {
    let rows = model.history_rows.iter().map(|item| {
        Row::new([
            item.title.as_str(),
            item.result.as_str(),
            item.finished_at.as_str(),
            item.output.as_str(),
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
            let style = if index == model.selected_setting {
                theme::selected()
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
    let help = [
        ("Navigate", "j/k or Up/Down  Tab"),
        ("Queue", "a Add job  Enter Details"),
        ("Job", "c Cancel  r Retry  o Open output"),
        ("Pages", "1 Queue  2 History  3 Settings  ? Help"),
        ("Application", "q Quit"),
    ];
    let lines = help
        .into_iter()
        .map(|(name, keys)| field_line(name, keys))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .block(theme::panel(" Help ", true))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_footer(frame: &mut Frame, area: Rect, model: &UiModel) {
    let status = model.status_message.as_deref().unwrap_or("Ready");
    let actions = match model.active_page {
        Page::Queue => " a Add   j/k Select   c Cancel   r Retry   o Open   ? Help   q Quit ",
        Page::AddJob => " Tab Field   Left/Right Choose   Enter Continue   Esc Back ",
        Page::History => " x Clear history   1 Queue   3 Settings   ? Help   q Quit ",
        Page::Settings => " j/k Select   e Edit   s Save   1 Queue   ? Help   q Quit ",
        Page::Help => " 1 Queue   2 History   3 Settings   ? Close help   q Quit ",
    };
    let lines = vec![
        Line::from(Span::styled(
            format!(" {status} "),
            Style::default().fg(theme::MUTED).bg(theme::PANEL),
        )),
        Line::from(Span::styled(
            actions,
            Style::default().fg(theme::KEY).bg(theme::PANEL),
        )),
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

fn transport_rail(current: usize) -> Paragraph<'static> {
    let stages = ["SOURCE", "PROBE", "QUEUE", "GET", "MERGE", "DONE"];
    let mut spans = Vec::new();
    for (index, stage) in stages.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" -> ", theme::faint()));
        }
        let style = if index < current {
            Style::default().fg(theme::SUCCESS)
        } else if index == current {
            Style::default()
                .fg(theme::FOCUS)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::faint()
        };
        spans.push(Span::styled(format!(" {stage} "), style));
    }
    Paragraph::new(Line::from(spans)).alignment(Alignment::Center)
}

fn field_line<'a>(name: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{name:<10}"), theme::muted()),
        Span::raw(value),
    ])
}

fn field_line_with_focus<'a>(name: &'a str, value: &'a str, focused: bool) -> Line<'a> {
    let marker = if focused { ">" } else { " " };
    let style = if focused {
        theme::selected()
    } else {
        Style::default().fg(theme::FOREGROUND).bg(theme::SURFACE)
    };
    Line::from(format!("{marker} {name:<9}{value}")).style(style)
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, text::Text};

    use super::*;

    fn render(model: &UiModel) -> String {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| draw(frame, model))
            .expect("page should render");
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

    fn model_for(page: Page) -> UiModel {
        UiModel {
            active_page: page,
            dependencies: DependencySummary {
                yt_dlp: "ready".into(),
                ffmpeg: "ready".into(),
            },
            queue_rows: vec![QueueRow {
                title: "Sample video".into(),
                status: "Downloading".into(),
                progress_percent: 42,
                speed: "4 MiB/s".into(),
                eta: "00:12".into(),
            }],
            selected_job: Some(JobDetails {
                title: "Sample video".into(),
                source: "https://example.com/watch?v=1".into(),
                format: "MP4".into(),
                output: "/Downloads/sample.mp4".into(),
                status: "GET".into(),
                progress_percent: 42,
                downloaded: "42 MiB".into(),
                total: "68 MiB".into(),
                speed: "4 MiB/s".into(),
                eta: "00:12".into(),
                transport_stage: 3,
                log_lines: vec!["Downloading media".into()],
                ..JobDetails::default()
            }),
            add_job: AddJobView {
                stage: AddStage::Review,
                source: "https://example.com/watch?v=1".into(),
                authentication: "Brave cookies".into(),
                profile: "Default".into(),
                mode: "Video / MP4".into(),
                quality: "4K".into(),
                output: "/Downloads".into(),
                ..AddJobView::default()
            },
            history_rows: vec![HistoryRow {
                title: "Finished video".into(),
                result: "Done".into(),
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
    fn renders_every_page_at_minimum_size() {
        for page in [
            Page::Queue,
            Page::AddJob,
            Page::History,
            Page::Settings,
            Page::Help,
        ] {
            render(&model_for(page));
        }
    }

    #[test]
    fn root_background_uses_background_role() {
        let backend = TestBackend::new(2, 1);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| {
                frame.render_widget(
                    Block::default().style(Style::default().bg(theme::BACKGROUND)),
                    frame.area(),
                );
            })
            .expect("background should render");

        assert_eq!(terminal.backend().buffer()[(0, 0)].bg, theme::BACKGROUND);
    }

    #[test]
    fn panel_roles_render_consistently() {
        let backend = TestBackend::new(12, 3);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| {
                let panels = Layout::horizontal([Constraint::Length(6), Constraint::Length(6)])
                    .split(frame.area());
                frame.render_widget(theme::panel("", true), panels[0]);
                frame.render_widget(theme::panel("", false), panels[1]);
            })
            .expect("panels should render");
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 0)].fg, theme::FOCUS);
        assert_eq!(buffer[(6, 0)].fg, theme::BORDER);
        assert_eq!(buffer[(1, 1)].bg, theme::SURFACE);
        assert_eq!(buffer[(7, 1)].bg, theme::SURFACE);
    }

    #[test]
    fn selected_role_uses_selection_background() {
        let backend = TestBackend::new(8, 1);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| {
                frame.render_widget(
                    Paragraph::new(Text::from("selected")).style(theme::selected()),
                    frame.area(),
                );
            })
            .expect("selection should render");
        let cell = &terminal.backend().buffer()[(0, 0)];

        assert_eq!(cell.fg, theme::FOREGROUND);
        assert_eq!(cell.bg, theme::SELECTION_BACKGROUND);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn page_chrome_and_queue_roles_match_theme() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal should be created");
        terminal
            .draw(|frame| draw(frame, &model_for(Page::Queue)))
            .expect("queue should render");
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(1, 0)].bg, theme::PANEL);
        assert_eq!(buffer[(1, 23)].bg, theme::PANEL);
        assert_eq!(buffer[(1, 4)].bg, theme::SURFACE);
        assert_eq!(buffer[(1, 4)].fg, theme::HEADING);
        assert_eq!(buffer[(1, 5)].bg, theme::SELECTION_BACKGROUND);
        assert_eq!(buffer[(1, 22)].fg, theme::KEY);
    }

    #[test]
    fn queue_shows_key_action_labels() {
        let output = render(&model_for(Page::Queue));
        assert!(output.contains("a Add"));
        assert!(output.contains("Cancel"));
        assert!(output.contains("Help"));
        assert!(output.contains("Quit"));
    }

    #[test]
    fn add_job_review_shows_selected_quality() {
        let output = render(&model_for(Page::AddJob));
        assert!(output.contains("REVIEW"));
        assert!(output.contains("4K"));
    }

    #[test]
    fn long_log_and_url_stay_inside_the_buffer() {
        let mut model = model_for(Page::Queue);
        let long = "x".repeat(20_000);
        let job = model
            .selected_job
            .as_mut()
            .expect("selected job should exist");
        job.source = format!("https://example.com/{long}");
        job.log_lines = vec![long];
        job.log_offset = u16::MAX;

        let output = render(&model);
        assert_eq!(output.lines().count(), 24);
        assert!(output.lines().all(|line| line.chars().count() <= 80));
    }
}
