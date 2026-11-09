//! Country detail screen: headline, population chart, history + forecast table.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Chart, Dataset, GraphType, HighlightSpacing, LegendPosition, Row, Table,
    TableState,
};

use super::table::{aligned, cell_style, format_cell, visible_columns};
use super::{Theme, draw_placeholder, format};
use crate::app::DetailState;
use crate::model::{CountryDetail, HistoryPoint};
use crate::tables::{Cell, ColKind, ColumnDef};

const COLUMNS: &[ColumnDef] = &[
    ColumnDef {
        title: "Year",
        csv: "year",
        width: 6,
        kind: ColKind::Text,
    },
    ColumnDef {
        title: "Population",
        csv: "population",
        width: 14,
        kind: ColKind::Int,
    },
    ColumnDef {
        title: "Yearly %",
        csv: "yearly_change_pct",
        width: 9,
        kind: ColKind::SignedPct,
    },
    ColumnDef {
        title: "Change",
        csv: "yearly_change",
        width: 11,
        kind: ColKind::Int,
    },
    ColumnDef {
        title: "Migrants",
        csv: "migrants_net",
        width: 10,
        kind: ColKind::Int,
    },
    ColumnDef {
        title: "Med. age",
        csv: "median_age",
        width: 8,
        kind: ColKind::Float(1),
    },
    ColumnDef {
        title: "Fert.",
        csv: "fertility_rate",
        width: 6,
        kind: ColKind::Float(2),
    },
    ColumnDef {
        title: "Density",
        csv: "density",
        width: 8,
        kind: ColKind::Int,
    },
    ColumnDef {
        title: "Urban %",
        csv: "urban_pop_pct",
        width: 9,
        kind: ColKind::Pct,
    },
    ColumnDef {
        title: "World %",
        csv: "world_share_pct",
        width: 8,
        kind: ColKind::Pct,
    },
    ColumnDef {
        title: "Rank",
        csv: "rank",
        width: 5,
        kind: ColKind::Int,
    },
    ColumnDef {
        title: "World pop.",
        csv: "world_population",
        width: 15,
        kind: ColKind::Int,
    },
];

fn cells(point: &HistoryPoint, forecast: bool) -> Vec<Cell> {
    let year = if forecast {
        format!("{}*", point.year)
    } else {
        point.year.to_string()
    };
    vec![
        Cell::Text(year),
        Cell::Num(Some(point.population as f64)),
        Cell::Num(point.yearly_change_pct),
        Cell::Num(point.yearly_change.map(|v| v as f64)),
        Cell::Num(point.migrants_net.map(|v| v as f64)),
        Cell::Num(point.median_age),
        Cell::Num(point.fertility_rate),
        Cell::Num(point.density),
        Cell::Num(point.urban_pop_pct),
        Cell::Num(point.world_share_pct),
        Cell::Num(point.rank.map(f64::from)),
        Cell::Num(point.world_population.map(|v| v as f64)),
    ]
}

/// Rows in display order: forecast (newest first), then history (newest first).
pub fn display_rows(detail: &CountryDetail) -> Vec<(&HistoryPoint, bool)> {
    let forecast = detail.forecast.iter().rev().map(|p| (p, true));
    let history = detail.history.iter().rev().map(|p| (p, false));
    forecast.chain(history).collect()
}

/// Draws the detail screen and returns rows per page.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut DetailState, theme: &Theme) -> usize {
    let Some(detail) = state.slot.data.as_ref() else {
        draw_placeholder(frame, area, &state.slot, &state.name, theme);
        return 1;
    };

    let chart_height = (area.height / 2).clamp(6, 14);
    let [header_area, chart_area, table_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(chart_height),
        Constraint::Min(3),
    ])
    .areas(area);

    draw_header(frame, header_area, detail, theme);
    draw_chart(frame, chart_area, detail, theme);

    let rows = display_rows(detail);
    let columns = &COLUMNS[..visible_columns(COLUMNS, table_area.width)];
    let header = Row::new(
        columns
            .iter()
            .map(|c| aligned(c.title.to_string(), c.kind, Style::default())),
    )
    .style(theme.fg(theme.accent).add_modifier(Modifier::BOLD));
    let body = rows.iter().map(|(point, forecast)| {
        let row_style = if *forecast {
            theme.fg(theme.muted).add_modifier(Modifier::ITALIC)
        } else {
            Style::default()
        };
        Row::new(
            columns
                .iter()
                .zip(cells(point, *forecast))
                .map(|(column, cell)| {
                    let style = if *forecast {
                        row_style
                    } else {
                        cell_style(&cell, column.kind, theme)
                    };
                    aligned(format_cell(&cell, column.kind), column.kind, style)
                }),
        )
    });
    let widget = Table::new(body, columns.iter().map(|c| Constraint::Length(c.width)))
        .header(header)
        .column_spacing(1)
        .row_highlight_style(theme.selected())
        .highlight_symbol("› ")
        .highlight_spacing(HighlightSpacing::Always);
    let mut table_state = TableState::default()
        .with_offset(state.offset)
        .with_selected(Some(state.selected.min(rows.len().saturating_sub(1))));
    frame.render_stateful_widget(widget, table_area, &mut table_state);
    state.offset = table_state.offset();

    table_area.height.saturating_sub(1).max(1) as usize
}

fn draw_header(frame: &mut Frame, area: Rect, detail: &CountryDetail, theme: &Theme) {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let sep = Span::styled("  ·  ", theme.fg(theme.muted));
    let mut spans = vec![
        Span::styled(
            format!(" {}", detail.name),
            theme.fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        sep.clone(),
        Span::raw("Population "),
        Span::styled(format::int(detail.population as f64), bold),
    ];
    if let Some(rank) = detail.rank {
        spans.extend([
            sep.clone(),
            Span::raw("Rank "),
            Span::styled(format!("#{rank}"), bold),
        ]);
    }
    if let Some(share) = detail.world_share_pct {
        spans.extend([
            sep,
            Span::styled(format::pct(share), bold),
            Span::raw(" of world"),
        ]);
    }

    let first = detail.history.first().map_or(0, |p| p.year);
    let last = detail.history.last().map_or(0, |p| p.year);
    let mut note = format!(" History {first}–{last}");
    if let Some(end) = detail.forecast.last() {
        note.push_str(&format!(" · forecast* to {}", end.year));
    }
    note.push_str(" · source: worldometers.info (UN World Population Prospects)");

    frame.render_widget(
        ratatui::widgets::Paragraph::new(vec![
            Line::from(spans),
            Line::styled(note, theme.fg(theme.muted)),
        ]),
        area,
    );
}

fn draw_chart(frame: &mut Frame, area: Rect, detail: &CountryDetail, theme: &Theme) {
    let history: Vec<(f64, f64)> = detail
        .history
        .iter()
        .map(|p| (f64::from(p.year), p.population as f64))
        .collect();
    // Start the forecast line at the last historical point so the lines join.
    let forecast: Vec<(f64, f64)> = detail
        .history
        .last()
        .into_iter()
        .chain(&detail.forecast)
        .map(|p| (f64::from(p.year), p.population as f64))
        .collect();

    let all = history.iter().chain(&forecast);
    let (mut x_min, mut x_max, mut y_min, mut y_max) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for (x, y) in all {
        x_min = x_min.min(*x);
        x_max = x_max.max(*x);
        y_min = y_min.min(*y);
        y_max = y_max.max(*y);
    }
    if x_min > x_max {
        return;
    }
    let pad = ((y_max - y_min) * 0.05).max(1.0);
    let (y_min, y_max) = ((y_min - pad).max(0.0), y_max + pad);
    if x_min == x_max {
        x_max += 1.0;
    }

    let mut datasets = vec![
        Dataset::default()
            .name("history")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(theme.fg(theme.accent))
            .data(&history),
    ];
    if forecast.len() > 1 {
        datasets.push(
            Dataset::default()
                .name("forecast")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(theme.fg(theme.muted))
                .data(&forecast),
        );
    }

    let x_mid = ((x_min + x_max) / 2.0).round();
    let axis_style = theme.fg(theme.muted);
    let chart = Chart::new(datasets)
        .block(
            Block::bordered()
                .border_style(axis_style)
                .title(" Population over time "),
        )
        .legend_position(Some(LegendPosition::TopLeft))
        .x_axis(
            Axis::default()
                .bounds([x_min, x_max])
                .style(axis_style)
                .labels([
                    format!("{x_min:.0}"),
                    format!("{x_mid:.0}"),
                    format!("{x_max:.0}"),
                ]),
        )
        .y_axis(
            Axis::default()
                .bounds([y_min, y_max])
                .style(axis_style)
                .labels([format::compact(y_min), format::compact(y_max)]),
        );
    frame.render_widget(chart, area);
}
