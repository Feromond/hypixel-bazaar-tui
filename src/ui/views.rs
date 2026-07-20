use crate::app::state::{App, SearchMode, prices};
use crate::util::{fmt_compact, fmt_count, fmt_pct, fmt_price};
use hypixel::models::skyblock::{BazaarProduct, BazaarQuickStatus};
use hypixel::util::market::BazaarFlip;
use ratatui::{
    prelude::*,
    symbols,
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, GraphType, List, ListItem, ListState,
        Paragraph, Row, Table, Wrap,
    },
};

/// Draws the search view, consisting of input, results, and status bar.
pub fn draw_search(frame: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Search input
            Constraint::Min(1),         // Results
            Constraint::Length(1),      // Status bar
        ])
        .split(frame.area());

    draw_search_input(frame, app, layout[0]);
    draw_search_results(frame, app, layout[1]);
    draw_status_bar(frame, app, layout[2]);
}

/// Draws the detail view for a selected product.
pub fn draw_detail(frame: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),      // Header
            Constraint::Percentage(40), // Quick Status & Orders
            Constraint::Percentage(60), // History Chart
        ])
        .split(frame.area());

    draw_detail_header(frame, app, layout[0]);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(layout[1]);

    if let Some(p) = app.current_product() {
        let flip = app.detail.product_id.as_deref().and_then(|id| app.flip(id));
        match p.quick_status.as_ref() {
            Some(q) => draw_quick_status(frame, q, flip, middle[0]),
            None => frame.render_widget(
                Paragraph::new("No quick status reported")
                    .block(Block::default().title("Quick Status").borders(Borders::ALL)),
                middle[0],
            ),
        }
        draw_orders(frame, p, middle[1]);
        draw_history_chart(frame, layout[2], app);
    } else {
        let msg = Paragraph::new("No product selected")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(msg, layout[1]);
    }
}

fn draw_search_input(frame: &mut Frame, app: &App, area: Rect) {
    let input_line = if app.search.input.is_empty() {
        Line::from(vec![Span::styled(
            "Type to search…",
            Style::default().fg(Color::DarkGray),
        )])
    } else {
        Line::from(Span::raw(app.search.input.as_str()))
    };

    let input_block = Block::default().title("Search").borders(Borders::ALL);
    let input = Paragraph::new(input_line)
        .block(input_block.clone())
        .wrap(Wrap { trim: true });
    
    frame.render_widget(input, area);

    if app.search.mode == SearchMode::Insert {
        let inner = input_block.inner(area);
        let x = inner.x.saturating_add(app.search.input.len() as u16);
        let y = inner.y;
        frame.set_cursor_position((x, y));
    }
}

fn draw_search_results(frame: &mut Frame, app: &mut App, area: Rect) {
    // Fixed-width numeric columns + borders (2) + highlight symbol ("▸ ", 2).
    const FIXED_COLS_WIDTH: u16 = 15 + 15 + 15 + 10 + 8 + 2 + 2;
    const MIN_NAME_WIDTH: usize = 16;
    let name_width = area
        .width
        .saturating_sub(FIXED_COLS_WIDTH)
        .max(MIN_NAME_WIDTH as u16) as usize;

    let items: Vec<ListItem> = app
        .search.filtered_indices
        .iter()
        .map(|i| {
            let item = &app.data.index[*i];
            if let Some(px) = app.data.products.get(&item.id).and_then(prices) {
                let flip = app.flip(&item.id);
                let (profit, margin, volume) = match flip {
                    Some(f) => (
                        fmt_price(f.profit_per_item),
                        fmt_pct(f.margin * 100.0),
                        fmt_compact(f.buy_moving_week.min(f.sell_moving_week)),
                    ),
                    None => ("—".to_string(), "—".to_string(), "—".to_string()),
                };
                let margin_color = flip.map_or(Color::DarkGray, |f| spread_color(f.margin * 100.0));

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:<name_width$}", truncate(&item.display, name_width)),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:>15}", fmt_price(px.instant_buy)),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        format!("{:>15}", fmt_price(px.instant_sell)),
                        Style::default().fg(Color::Red),
                    ),
                    Span::styled(
                        format!("{profit:>15}"),
                        Style::default().fg(if flip.is_some() {
                            Color::Yellow
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::styled(format!("{margin:>10}"), Style::default().fg(margin_color)),
                    Span::styled(
                        format!("{volume:>8}"),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(line)
            } else {
                let styled = Line::from(Span::styled(
                    item.display.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
                ListItem::new(styled)
            }
        })
        .collect();

    let mut list_state = ListState::default();
    if !app.search.filtered_indices.is_empty() {
        list_state.select(Some(app.search.selected_index));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(Line::from(vec![
                    Span::styled("Products ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("({} results, ", app.search.filtered_indices.len()),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        if app.search.sort_by_profit {
                            "by flip profit"
                        } else {
                            "by relevance"
                        },
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(")", Style::default().fg(Color::Gray)),
                ]))
                .title_bottom(
                    Line::from(Span::styled(
                        format!(
                            "{:<name_width$}{:>15}{:>15}{:>15}{:>10}{:>8}",
                            "", "buy", "sell", "profit", "margin", "vol/wk"
                        ),
                        Style::default().fg(Color::DarkGray),
                    ))
                    .left_aligned(),
                )
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("▸ ");
        
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode = if app.search.mode == SearchMode::Insert { "Insert" } else { "Navigate" };
    let hints = "Esc quit • Enter detail • ↑/↓ navigate • Ctrl+S sort";
    let status_line = Line::from(vec![
        Span::styled(app.status.clone(), Style::default().fg(Color::Gray)),
        Span::raw("   "),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
        Span::raw("   |  Data "),
        Span::styled(age_label(app.data.last_updated), Style::default().fg(Color::DarkGray)),
        Span::raw("   |  Mode: "),
        Span::styled(mode, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    
    let status = Paragraph::new(status_line).wrap(Wrap { trim: true });
    frame.render_widget(status, area);
}

fn draw_detail_header(frame: &mut Frame, app: &App, area: Rect) {
    let Some(id) = &app.detail.product_id else {
        frame.render_widget(
            Paragraph::new("Detail").style(Style::default().fg(Color::Yellow)),
            area,
        );
        return;
    };

    let toggle = |on: bool, label: &str| {
        Span::styled(
            format!("{label} "),
            Style::default().fg(if on { Color::Yellow } else { Color::DarkGray }),
        )
    };

    let header = Line::from(vec![
        Span::styled(
            format!("{id}   "),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::styled("b=back  r=refresh   ", Style::default().fg(Color::DarkGray)),
        toggle(app.detail.show_percent, "p=%"),
        toggle(app.detail.show_sma, "m=sma"),
        toggle(app.detail.show_midline, "g=mean"),
    ]);
    frame.render_widget(Paragraph::new(header), area);
}

fn draw_quick_status(
    frame: &mut Frame,
    q: &BazaarQuickStatus,
    flip: Option<&BazaarFlip>,
    area: Rect,
) {
    let spread = q.buy_price - q.sell_price;
    let spread_pct = if q.sell_price.abs() > f64::EPSILON {
        spread / q.sell_price * 100.0
    } else {
        0.0
    };

    let (flip_profit, flip_margin) = match flip {
        Some(f) => (
            Cell::from(fmt_price(f.profit_per_item)).style(Style::default().fg(Color::Yellow)),
            Cell::from(fmt_pct(f.margin * 100.0))
                .style(Style::default().fg(spread_color(f.margin * 100.0))),
        ),
        None => {
            let dim = Style::default().fg(Color::DarkGray);
            (
                Cell::from("not viable").style(dim),
                Cell::from("—").style(dim),
            )
        }
    };

    let rows = vec![
        Row::new(vec![
            Cell::from("Instant Buy"),
            colored_price(q.buy_price, Color::Green),
        ]),
        Row::new(vec![
            Cell::from("Instant Sell"),
            colored_price(q.sell_price, Color::Red),
        ]),
        Row::new(vec![
            Cell::from("Spread"),
            colored_price(spread, Color::Yellow),
        ]),
        Row::new(vec![
            Cell::from("Spread %"),
            Cell::from(fmt_pct(spread_pct)).style(Style::default().fg(spread_color(spread_pct))),
        ]),
        Row::new(vec![Cell::from("Flip Profit"), flip_profit]),
        Row::new(vec![Cell::from("Flip Margin"), flip_margin]),
        Row::new(vec![Cell::from(""), Cell::from("")]),
        Row::new(vec![
            Cell::from("Buy Vol"),
            Cell::from(fmt_count(q.buy_volume)),
        ]),
        Row::new(vec![
            Cell::from("Sell Vol"),
            Cell::from(fmt_count(q.sell_volume)),
        ]),
        Row::new(vec![
            Cell::from("Buy Move/Wk"),
            Cell::from(fmt_count(q.buy_moving_week)),
        ]),
        Row::new(vec![
            Cell::from("Sell Move/Wk"),
            Cell::from(fmt_count(q.sell_moving_week)),
        ]),
        Row::new(vec![
            Cell::from("Buy Orders"),
            Cell::from(fmt_count(q.buy_orders)),
        ]),
        Row::new(vec![
            Cell::from("Sell Orders"),
            Cell::from(fmt_count(q.sell_orders)),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(14), Constraint::Min(10)])
        .block(Block::default().title("Quick Status").borders(Borders::ALL));

    frame.render_widget(table, area);
}

fn draw_orders(frame: &mut Frame, p: &BazaarProduct, area: Rect) {
    // buy_summary is the ask side, sell_summary the bid side.
    let buys = p.buy_summary.iter().take(5).map(|o| {
        Row::new(vec![
            colored_price(o.price_per_unit, Color::Green),
            Cell::from(fmt_count(o.amount)),
            Cell::from(fmt_count(o.orders)),
        ])
    });
    let sells = p.sell_summary.iter().take(5).map(|o| {
        Row::new(vec![
            colored_price(o.price_per_unit, Color::Red),
            Cell::from(fmt_count(o.amount)),
            Cell::from(fmt_count(o.orders)),
        ])
    });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let header_style = Style::default().add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::from("Price"),
        Cell::from("Amt"),
        Cell::from("#"),
    ]).style(header_style);

    let widths = [
        Constraint::Length(14),
        Constraint::Length(12),
        Constraint::Length(8),
    ];

    let buy_table = Table::new(buys, widths)
        .header(header.clone())
        .block(
            Block::default()
                .title("Instant Buy — best asks")
                .borders(Borders::ALL),
        );

    frame.render_widget(buy_table, chunks[0]);

    let sell_table = Table::new(sells, widths).header(header).block(
        Block::default()
            .title("Instant Sell — best bids")
            .borders(Borders::ALL),
    );

    frame.render_widget(sell_table, chunks[1]);
}

fn colored_price(v: f64, color: Color) -> Cell<'static> {
    Cell::from(fmt_price(v)).style(Style::default().fg(color))
}

/// Green above 5%, yellow above 1%; below that is noise after the 1.25% tax.
fn spread_color(pct: f64) -> Color {
    if pct >= 5.0 {
        Color::Green
    } else if pct >= 1.0 {
        Color::Yellow
    } else {
        Color::DarkGray
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn age_label(last_updated_ms: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    match now_ms.checked_sub(last_updated_ms) {
        Some(ms) if (0..60_000).contains(&ms) => format!("{}s old", ms / 1000),
        Some(ms) if ms >= 60_000 => format!("{}m old", ms / 60_000),
        _ => "just now".to_string(),
    }
}


const SMA_WINDOW: usize = 5;

/// `(seconds since first sample, value)` points.
type Series = Vec<(f64, f64)>;

/// Draws price history as two panes, each scaled to its own series so small
/// moves stay visible despite the ask sitting well above the bid.
fn draw_history_chart(frame: &mut Frame, area: Rect, app: &App) {
    let (pts_buy, pts_sell) = history_series(app);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    draw_history_legend(frame, chunks[0], app);

    let max_x = pts_buy.last().map(|p| p.0).unwrap_or(1.0).max(1.0);
    let percent = app.detail.show_percent;

    let (buy_sma, sell_sma) = if app.detail.show_sma {
        (sma(&pts_buy, SMA_WINDOW), sma(&pts_sell, SMA_WINDOW))
    } else {
        (Vec::new(), Vec::new())
    };
    let (buy_mid, sell_mid) = if app.detail.show_midline {
        (mean_line(&pts_buy, max_x), mean_line(&pts_sell, max_x))
    } else {
        (Vec::new(), Vec::new())
    };

    draw_price_pane(
        frame,
        chunks[1],
        PaneSpec {
            title: "Instant Buy (ask)",
            color: Color::Green,
            sma_color: Color::LightGreen,
            pts: &pts_buy,
            sma: &buy_sma,
            mid: &buy_mid,
            max_x,
            percent,
        },
    );
    draw_price_pane(
        frame,
        chunks[2],
        PaneSpec {
            title: "Instant Sell (bid)",
            color: Color::Red,
            sma_color: Color::LightRed,
            pts: &pts_sell,
            sma: &sell_sma,
            mid: &sell_mid,
            max_x,
            percent,
        },
    );
}

struct PaneSpec<'a> {
    title: &'a str,
    color: Color,
    sma_color: Color,
    pts: &'a [(f64, f64)],
    sma: &'a [(f64, f64)],
    mid: &'a [(f64, f64)],
    max_x: f64,
    percent: bool,
}

fn draw_price_pane(frame: &mut Frame, area: Rect, spec: PaneSpec<'_>) {
    let fmt_y = |v: f64| if spec.percent { fmt_pct(v) } else { fmt_price(v) };

    let current = spec.pts.last().map(|p| p.1);
    let mut title = vec![
        Span::styled(
            format!("{} ", spec.title),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            current.map(fmt_y).unwrap_or_else(|| "—".to_string()),
            Style::default().fg(spec.color),
        ),
        Span::raw("   "),
        Span::styled("──", Style::default().fg(spec.color)),
        Span::styled(" price", Style::default().fg(Color::DarkGray)),
    ];
    if !spec.sma.is_empty() {
        title.push(Span::styled("  ──", Style::default().fg(spec.sma_color)));
        title.push(Span::styled(
            format!(" SMA({SMA_WINDOW})"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    if !spec.mid.is_empty() {
        title.push(Span::styled("  ──", Style::default().fg(Color::Gray)));
        title.push(Span::styled(" mean", Style::default().fg(Color::DarkGray)));
    }

    let block = Block::default().title(Line::from(title)).borders(Borders::ALL);

    if spec.pts.len() < 2 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "collecting samples…",
                Style::default().fg(Color::DarkGray),
            ))
            .block(block),
            area,
        );
        return;
    }

    let [y_min, y_max] = auto_bounds(spec.pts);

    let mut datasets = vec![
        Dataset::default()
            .name("price")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(spec.color))
            .data(spec.pts),
    ];
    if !spec.sma.is_empty() {
        datasets.push(
            Dataset::default()
                .name(format!("SMA({SMA_WINDOW})"))
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(spec.sma_color))
                .data(spec.sma),
        );
    }
    if !spec.mid.is_empty() {
        datasets.push(
            Dataset::default()
                .name("mean")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Gray))
                .data(spec.mid),
        );
    }

    let y_mid = (y_min + y_max) / 2.0;
    let chart = Chart::new(datasets)
        .block(block)
        .legend_position(None)
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, spec.max_x])
                .labels(vec![
                    Span::raw("0s"),
                    Span::raw(format!("{:.0}s", spec.max_x / 2.0)),
                    Span::raw(format!("{:.0}s", spec.max_x)),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([y_min, y_max])
                .labels(vec![
                    Span::raw(fmt_y(y_min)),
                    Span::raw(fmt_y(y_mid)),
                    Span::raw(fmt_y(y_max)),
                ]),
        );

    frame.render_widget(chart, area);
}

/// Summary line above the panes; always absolute, even in percent mode.
fn draw_history_legend(frame: &mut Frame, area: Rect, app: &App) {
    let px = app.current_product().and_then(prices);

    let mut spans = vec![
        Span::styled("● ", Style::default().fg(Color::Green)),
        Span::styled("buy ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            px.map(|p| fmt_price(p.instant_buy))
                .unwrap_or_else(|| "—".into()),
            Style::default().fg(Color::Green),
        ),
        Span::raw("   "),
        Span::styled("● ", Style::default().fg(Color::Red)),
        Span::styled("sell ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            px.map(|p| fmt_price(p.instant_sell))
                .unwrap_or_else(|| "—".into()),
            Style::default().fg(Color::Red),
        ),
        Span::raw("   "),
        Span::styled("spread ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            px.map(|p| format!("{} ({})", fmt_price(p.spread()), fmt_pct(p.spread_pct())))
                .unwrap_or_else(|| "—".into()),
            Style::default().fg(px.map_or(Color::Gray, |p| spread_color(p.spread_pct()))),
        ),
    ];

    if app.detail.show_percent {
        spans.push(Span::styled(
            "   [% change from first sample]",
            Style::default().fg(Color::DarkGray),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Sample history as two series: absolute coins, or percent change from first.
fn history_series(app: &App) -> (Series, Series) {
    let mut pts_buy = Vec::with_capacity(app.detail.history.len());
    let mut pts_sell = Vec::with_capacity(app.detail.history.len());

    let Some(&(t0, b0, s0)) = app.detail.history.front() else {
        return (pts_buy, pts_sell);
    };

    for (t, b, s) in app.detail.history.iter() {
        let x = (*t - t0).as_secs_f64();
        if app.detail.show_percent {
            let rebase =
                |v: f64, base: f64| if base != 0.0 { (v - base) / base * 100.0 } else { 0.0 };
            pts_buy.push((x, rebase(*b, b0)));
            pts_sell.push((x, rebase(*s, s0)));
        } else {
            pts_buy.push((x, *b));
            pts_sell.push((x, *s));
        }
    }

    (pts_buy, pts_sell)
}

fn sma(pts: &[(f64, f64)], k: usize) -> Series {
    if k == 0 || pts.len() < k {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(pts.len() - k + 1);
    let mut sum = 0.0;
    for (i, p) in pts.iter().enumerate() {
        sum += p.1;
        if i >= k {
            sum -= pts[i - k].1;
        }
        if i + 1 >= k {
            out.push((p.0, sum / k as f64));
        }
    }
    out
}

fn mean_line(pts: &[(f64, f64)], max_x: f64) -> Series {
    if pts.is_empty() {
        return Vec::new();
    }
    let mean = pts.iter().map(|p| p.1).sum::<f64>() / pts.len() as f64;
    vec![(0.0, mean), (max_x, mean)]
}

/// Padded y bounds for one series.
fn auto_bounds(pts: &[(f64, f64)]) -> [f64; 2] {
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for (_, v) in pts {
        min_v = min_v.min(*v);
        max_v = max_v.max(*v);
    }
    if !min_v.is_finite() || !max_v.is_finite() {
        return [0.0, 1.0];
    }

    let span = max_v - min_v;
    if span <= f64::EPSILON {
        let pad = if min_v.abs() > f64::EPSILON {
            min_v.abs() * 0.001
        } else {
            1.0
        };
        [min_v - pad, max_v + pad]
    } else {
        let pad = span * 0.08;
        [min_v - pad, max_v + pad]
    }
}
