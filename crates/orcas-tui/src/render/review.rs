use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::AppState;
use crate::view_model;

use super::shared::{
    focus_block_style, key_hint_style, label_style, metadata_style, panel_title_style,
    render_panel_with_focus, row_style, selection_marker, selection_marker_style,
    status_text_style, value_style,
};

pub(super) fn render_surface(frame: &mut Frame<'_>, state: &AppState) {
    let header_height = if frame.area().height < 34 { 5 } else { 6 };
    let footer_height = if frame.area().height < 34 { 7 } else { 8 };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(12),
            Constraint::Length(footer_height),
        ])
        .split(frame.area());

    let review = view_model::review_view(state);
    render_header(frame, review.header.clone(), layout[0]);
    render_body(frame, review.queue, review.detail_panel, layout[1]);
    frame.render_widget(render_footer(review.footer), layout[2]);
}

fn render_header(frame: &mut Frame<'_>, header: view_model::ReviewHeaderViewModel, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Length(20),
            Constraint::Percentage(38),
        ])
        .split(area);
    frame.render_widget(render_status_segments(header.status_segments), sections[0]);
    frame.render_widget(render_program_tabs(header.program_tabs), sections[1]);
    frame.render_widget(render_summary(header.summary_lines), sections[2]);
}

fn render_body(
    frame: &mut Frame<'_>,
    queue: view_model::ReviewQueueViewModel,
    detail_panel: view_model::PanelViewModel,
    area: Rect,
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);
    frame.render_widget(render_queue(queue, columns[0]), columns[0]);
    frame.render_widget(
        render_panel_with_focus(detail_panel, false, false),
        columns[1],
    );
}

fn render_status_segments(
    segments: Vec<view_model::MainStatusSegmentViewModel>,
) -> Paragraph<'static> {
    let mut spans = Vec::new();
    for (index, segment) in segments.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ", metadata_style()));
        }
        spans.push(Span::styled(format!("{} ", segment.label), label_style()));
        spans.push(Span::styled(
            format!("[{}]", segment.value),
            status_text_style(&segment.value),
        ));
    }
    Paragraph::new(Text::from(vec![Line::from(spans)]))
        .block(
            Block::default()
                .title(Line::styled("Status", panel_title_style(true)))
                .borders(Borders::ALL)
                .border_style(focus_block_style(true)),
        )
        .wrap(Wrap { trim: true })
}

fn render_program_tabs(tabs: Vec<view_model::ProgramTabViewModel>) -> Paragraph<'static> {
    let tabs = tabs
        .into_iter()
        .flat_map(|tab| {
            let mut spans = vec![Span::styled(
                if tab.selected {
                    format!("[{}]", tab.label)
                } else {
                    tab.label
                },
                if tab.selected {
                    key_hint_style()
                } else {
                    value_style()
                },
            )];
            spans.push(Span::styled(" ", metadata_style()));
            spans
        })
        .collect::<Vec<_>>();
    Paragraph::new(Text::from(vec![Line::from(tabs)]))
        .block(
            Block::default()
                .title(Line::styled("Program", panel_title_style(true)))
                .borders(Borders::ALL)
                .border_style(focus_block_style(true)),
        )
        .wrap(Wrap { trim: true })
}

fn render_summary(lines: Vec<String>) -> Paragraph<'static> {
    Paragraph::new(Text::from(
        lines
            .into_iter()
            .map(|line| Line::styled(line, metadata_style()))
            .collect::<Vec<_>>(),
    ))
    .block(
        Block::default()
            .title(Line::styled("Review Summary", panel_title_style(true)))
            .borders(Borders::ALL)
            .border_style(focus_block_style(true)),
    )
    .wrap(Wrap { trim: true })
}

fn render_queue(queue: view_model::ReviewQueueViewModel, area: Rect) -> Paragraph<'static> {
    let visible_rows = area.height.saturating_sub(2) as usize;
    let total_rows = queue.rows.len();
    let offset = queue
        .scroll_offset
        .min(total_rows.saturating_sub(visible_rows));

    let lines = if queue.rows.is_empty() {
        vec![Line::styled(
            "No review items are queued.",
            metadata_style(),
        )]
    } else {
        queue
            .rows
            .into_iter()
            .skip(offset)
            .take(visible_rows.max(1))
            .map(|row| {
                let mut spans = vec![
                    Span::styled(
                        selection_marker(row.selected, true),
                        selection_marker_style(row.selected, true),
                    ),
                    Span::styled(" ", metadata_style()),
                    Span::styled(review_kind_label(row.kind), metadata_style()),
                    Span::styled(" ", metadata_style()),
                    Span::styled(row.label, row_style(row.selected, true)),
                ];
                for badge in row.badges {
                    if badge == "-" || badge.is_empty() {
                        continue;
                    }
                    spans.push(Span::styled(" ", metadata_style()));
                    spans.push(Span::styled(
                        format!("[{}]", badge),
                        status_text_style(&badge),
                    ));
                }
                if let Some(secondary) = row.secondary {
                    spans.push(Span::styled(" ", metadata_style()));
                    spans.push(Span::styled(secondary, metadata_style()));
                }
                Line::from(spans)
            })
            .collect::<Vec<_>>()
    };

    Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Line::styled(
                    format!(
                        "Review Queue {}",
                        queue
                            .selected_index
                            .map(|index| format!("({}/{})", index + 1, total_rows.max(1)))
                            .unwrap_or_else(|| "(0/0)".to_string())
                    ),
                    panel_title_style(true),
                ))
                .borders(Borders::ALL)
                .border_style(focus_block_style(true)),
        )
        .wrap(Wrap { trim: true })
}

fn render_footer(footer: view_model::ReviewFooterViewModel) -> Paragraph<'static> {
    let mut lines = footer
        .lines
        .into_iter()
        .map(|line| Line::styled(line, value_style()))
        .collect::<Vec<_>>();
    lines.push(Line::styled(String::new(), metadata_style()));
    lines.push(Line::from(vec![
        Span::styled("keys: ", label_style()),
        Span::styled(footer.hint_line, key_hint_style()),
    ]));
    Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(Line::styled(footer.title, panel_title_style(true)))
                .borders(Borders::ALL)
                .border_style(focus_block_style(true)),
        )
        .wrap(Wrap { trim: true })
}

fn review_kind_label(kind: view_model::ReviewRowKind) -> &'static str {
    match kind {
        view_model::ReviewRowKind::Proposal => "proposal",
        view_model::ReviewRowKind::Decision => "decision",
        view_model::ReviewRowKind::Failure => "failure",
        view_model::ReviewRowKind::ReviewRequired => "review",
    }
}
