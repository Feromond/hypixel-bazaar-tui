use crate::app::state::{App, SearchMode};
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

    // Split middle section into Quick Status (left) and Orders (right)
    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(layout[1]);

    if let Some(p) = app.current_product() {
        draw_quick_status(frame, &p.quick_status, middle[0]);
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
    let items: Vec<ListItem> = app
        .search.filtered_indices
        .iter()
        .map(|i| {
            let item = &app.data.index[*i];
            if let Some(p) = app.data.products.get(&item.id) {
                let buy = p.quick_status.buy_price;
                let sell = p.quick_status.sell_price;
                let spread = sell - buy;
                let line = Line::from(vec![
                    Span::styled(
                        item.display.clone(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  ["),
                    Span::styled("B:", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:.1}", buy), Style::default().fg(Color::Green)),
                    Span::raw("  "),
                    Span::styled("S:", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:.1}", sell), Style::default().fg(Color::Red)),
                    Span::raw("  "),
                    Span::styled("Δ:", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:+.1}", spread),
                        Style::default().fg(if spread >= 0.0 { Color::Green } else { Color::Red }),
                    ),
                    Span::raw("]"),
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
                    Span::raw("(Enter to open) – "),
                    Span::styled(
                        format!("{} results", app.search.filtered_indices.len()),
                        Style::default().fg(Color::Gray),
                    ),
                ]))
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
        Span::raw("   |  Last Updated: "),
        Span::styled(app.data.last_updated.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw("   |  Mode: "),
        Span::styled(mode, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    
    let status = Paragraph::new(status_line).wrap(Wrap { trim: true });
    frame.render_widget(status, area);
}

fn draw_detail_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = match &app.detail.product_id {
        Some(id) => format!("Detail: {id}   (b=back, r=refresh)"),
        None => "Detail".to_string(),
    };
    let header = Paragraph::new(title).style(Style::default().fg(Color::Yellow));
    frame.render_widget(header, area);
}

fn draw_quick_status(frame: &mut Frame, q: &crate::api::models::QuickStatus, area: Rect) {
    let buy_cell = colored_price(q.buy_price, Color::Green);
    let sell_cell = colored_price(q.sell_price, Color::Red);
    let spread = (q.sell_price - q.buy_price).max(0.0);
    let spread_cell = colored_price(
        spread,
        if spread >= 0.0 { Color::Green } else { Color::Red },
    );

    let rows = vec![
        Row::new(vec![Cell::from("Product ID"), Cell::from(q.product_id.clone())]),
        Row::new(vec![Cell::from("Buy Price"), buy_cell]),
        Row::new(vec![Cell::from("Sell Price"), sell_cell]),
        Row::new(vec![Cell::from("Spread"), spread_cell]),
        Row::new(vec![Cell::from("Buy Vol"), Cell::from(q.buy_volume.to_string())]),
        Row::new(vec![Cell::from("Sell Vol"), Cell::from(q.sell_volume.to_string())]),
        Row::new(vec![Cell::from("Buy Move/Wk"), Cell::from(q.buy_moving_week.to_string())]),
        Row::new(vec![Cell::from("Sell Move/Wk"), Cell::from(q.sell_moving_week.to_string())]),
        Row::new(vec![Cell::from("Buy Orders"), Cell::from(q.buy_orders.to_string())]),
        Row::new(vec![Cell::from("Sell Orders"), Cell::from(q.sell_orders.to_string())]),
    ];
    
    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(10)])
        .block(Block::default().title("Quick Status").borders(Borders::ALL));
        
    frame.render_widget(table, area);
}

fn draw_orders(frame: &mut Frame, p: &crate::api::models::Product, area: Rect) {
    // Orders (top 5 buy/sell)
    let buys = p.buy_summary.iter().take(5).map(|o| {
        Row::new(vec![
            colored_price(o.price_per_unit, Color::Green),
            Cell::from(o.amount.to_string()),
            Cell::from(o.orders.to_string()),
        ])
    });
    let sells = p.sell_summary.iter().take(5).map(|o| {
        Row::new(vec![
            colored_price(o.price_per_unit, Color::Red),
            Cell::from(o.amount.to_string()),
            Cell::from(o.orders.to_string()),
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

    let buy_table = Table::new(
        buys,
        [Constraint::Length(12), Constraint::Length(10), Constraint::Length(8)],
    )
    .header(header.clone())
    .block(Block::default().title("Top Buys").borders(Borders::ALL));
    
    frame.render_widget(buy_table, chunks[0]);

    let sell_table = Table::new(
        sells,
        [Constraint::Length(12), Constraint::Length(10), Constraint::Length(8)],
    )
    .header(header)
    .block(Block::default().title("Top Sells").borders(Borders::ALL));
    
    frame.render_widget(sell_table, chunks[1]);
}

fn colored_price(v: f64, color: Color) -> Cell<'static> {
    Cell::from(format!("{:.1}", v)).style(Style::default().fg(color))
}

fn draw_history_chart(frame: &mut Frame, area: Rect, app: &App) {
    let mut pts_buy: Vec<(f64, f64)> = Vec::new();
    let mut pts_sell: Vec<(f64, f64)> = Vec::new();

    if app.detail.history.len() >= 2 {
        let t0 = app.detail.history.front().unwrap().0;
        if app.detail.show_percent {
            let (b0, s0) = (app.detail.history.front().unwrap().1, app.detail.history.front().unwrap().2);
            for (t, b, s) in app.detail.history.iter() {
                let x = (*t - t0).as_secs_f64();
                let bp = if b0 != 0.0 { (b - b0) / b0 * 100.0 } else { 0.0 };
                let sp = if s0 != 0.0 { (s - s0) / s0 * 100.0 } else { 0.0 };
                pts_buy.push((x, bp));
                pts_sell.push((x, sp));
            }
        } else {
            for (t, b, s) in app.detail.history.iter() {
                let x = (*t - t0).as_secs_f64();
                pts_buy.push((x, *b));
                pts_sell.push((x, *s));
            }
        }
    }

    // SMA
    let sma = |pts: &[(f64, f64)], k: usize| -> Vec<(f64, f64)> {
        if pts.len() < k { return Vec::new(); }
        let mut out = Vec::with_capacity(pts.len() - k + 1);
        let mut sum = 0.0;
        for i in 0..pts.len() {
            sum += pts[i].1;
            if i >= k {
                sum -= pts[i - k].1;
            }
            if i + 1 >= k {
                out.push((pts[i].0, sum / (k as f64)));
            }
        }
        out
    };
    let pts_buy_sma = if app.detail.show_sma { sma(&pts_buy, 5) } else { Vec::new() };
    let pts_sell_sma = if app.detail.show_sma { sma(&pts_sell, 5) } else { Vec::new() };

    let datasets = if pts_buy.is_empty() {
        vec![]
    } else {
        let mut v = vec![
            Dataset::default()
                .name("Buy")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Green))
                .data(&pts_buy),
            Dataset::default()
                .name("Sell")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Red))
                .data(&pts_sell),
        ];
        if !pts_buy_sma.is_empty() {
            v.push(Dataset::default()
                .name("Buy SMA(5)")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::LightGreen))
                .data(&pts_buy_sma));
        }
        if !pts_sell_sma.is_empty() {
            v.push(Dataset::default()
                .name("Sell SMA(5)")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::LightRed))
                .data(&pts_sell_sma));
        }
        v
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3)])
        .split(area);

    // Legend
    let last_buy = pts_buy.last().map(|p| p.1);
    let last_sell = pts_sell.last().map(|p| p.1);
    let spread = match (last_buy, last_sell) {
        (Some(b), Some(s)) => Some((s - b, if b != 0.0 { (s - b) / b * 100.0 } else { 0.0 })),
        _ => None,
    };
    let legend_line = Line::from(vec![
        Span::styled("● ", Style::default().fg(Color::Green)),
        Span::raw("Buy "),
        Span::styled(
            format!("{}", last_buy.map(|v| format!("{:.1}", v)).unwrap_or("-".into())),
            Style::default().fg(Color::Green),
        ),
        Span::raw("   "),
        Span::styled("● ", Style::default().fg(Color::Red)),
        Span::raw("Sell "),
        Span::styled(
            format!("{}", last_sell.map(|v| format!("{:.1}", v)).unwrap_or("-".into())),
            Style::default().fg(Color::Red),
        ),
        Span::raw("   "),
        Span::raw("Spread "),
        Span::styled(
            spread.map(|(d, p)| format!("{:+.1} ({:+.2}%)", d, p)).unwrap_or("-".into()),
            Style::default().fg(Color::Yellow),
        ),
    ]);
    frame.render_widget(Paragraph::new(legend_line), chunks[0]);

    // Chart
    let max_x = pts_buy.last().map(|p| p.0).unwrap_or(1.0).max(1.0);
    let x_labels = vec![
        Span::raw("0"),
        Span::raw(format!("{:.0}", max_x)),
    ];

    let [y_min, y_max] = auto_bounds(&pts_buy, &pts_sell);
    let y_labels = vec![
        Span::raw(format!("{:.1}", y_min)),
        Span::raw(format!("{:.1}", y_max)),
    ];

    let title = match &app.detail.product_id {
        Some(id) => format!("Price History: {}", id),
        None => "Price History".to_string(),
    };

    let chart = Chart::new(datasets)
        .block(Block::default().title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD))).borders(Borders::ALL))
        .x_axis(Axis::default().bounds([0.0, max_x]).labels(x_labels))
        .y_axis(Axis::default().bounds([y_min, y_max]).labels(y_labels));

    frame.render_widget(chart, chunks[1]);
}

fn auto_bounds(b: &[(f64, f64)], s: &[(f64, f64)]) -> [f64; 2] {
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for (_, v) in b.iter().chain(s.iter()) {
        min_v = min_v.min(*v);
        max_v = max_v.max(*v);
    }
    if !min_v.is_finite() || !max_v.is_finite() || (max_v - min_v).abs() < 1e-6 {
        [0.0, 1.0]
    } else {
        let pad = (max_v - min_v) * 0.05;
        [min_v - pad, max_v + pad]
    }
}
