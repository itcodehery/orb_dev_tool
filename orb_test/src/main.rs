use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
    Terminal,
};
use std::{error::Error, io, collections::HashSet, process::{Command, Stdio}, time::Duration};
use std::fs;

struct App {
    all_tools: Vec<String>,
    tools: Vec<String>,
    search_query: String,
    search_mode: bool,
    selected_index: usize,
    scroll_offset: usize,
}

impl App {
    fn new() -> Self {
        let mut all_tools = discover_tools();
        if all_tools.is_empty() {
            all_tools.push("echo".to_string());
        }
        
        let tools = all_tools.clone();

        Self {
            all_tools,
            tools,
            search_query: String::new(),
            search_mode: false,
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    fn update_search(&mut self) {
        let query = self.search_query.to_lowercase();
        self.tools = self.all_tools.iter().filter(|t| t.to_lowercase().contains(&query)).cloned().collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    fn move_up(&mut self) {
        if self.selected_index >= 2 {
            self.selected_index -= 2;
        } else {
            self.selected_index = 0;
        }
        self.adjust_scroll();
    }

    fn move_down(&mut self) {
        if self.selected_index + 2 < self.tools.len() {
            self.selected_index += 2;
        } else {
            self.selected_index = self.tools.len().saturating_sub(1);
        }
        self.adjust_scroll();
    }

    fn move_left(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        self.adjust_scroll();
    }

    fn move_right(&mut self) {
        if self.selected_index + 1 < self.tools.len() {
            self.selected_index += 1;
        }
        self.adjust_scroll();
    }

    fn adjust_scroll(&mut self) {
        if self.tools.is_empty() {
            return;
        }
        let row = self.selected_index / 2;
        let visible_start_row = self.scroll_offset / 2;
        
        if row < visible_start_row {
            self.scroll_offset = row * 2;
        } else if row >= visible_start_row + 2 {
            self.scroll_offset = (row - 1) * 2;
        }
    }

    fn run_selected(&mut self) {
        if let Some(tool) = self.tools.get(self.selected_index) {
            let tool_clone = tool.clone();
            std::thread::spawn(move || {
                let mut cmd = Command::new(&tool_clone);
                cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
                let _ = cmd.spawn(); // Just spawn and ignore since we removed logging pane
            });
        }
    }
}

fn discover_tools() -> Vec<String> {
    let mut tools_set = HashSet::new();
    if let Ok(path_var) = std::env::var("PATH") {
        for path in path_var.split(':') {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.filter_map(Result::ok) {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() || file_type.is_symlink() {
                            if let Ok(name) = entry.file_name().into_string() {
                                if name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                                    tools_set.insert(name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let mut tools: Vec<String> = tools_set.into_iter().collect();
    tools.sort();
    tools
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> where io::Error: From<B::Error> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if app.search_mode {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => app.search_mode = false,
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.update_search();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.update_search();
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::F(10) => return Ok(()),
                        KeyCode::Char('s') | KeyCode::Char('/') => app.search_mode = true,
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Left | KeyCode::Char('h') => app.move_left(),
                        KeyCode::Right | KeyCode::Char('l') => app.move_right(),
                        KeyCode::Enter => app.run_selected(),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(1)].as_ref())
        .split(size);
        
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(chunks[1]);

    // Header
    let header_text = vec![
        Line::from(vec![
            Span::styled("ORB_v1.0.4  ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("UPTIME: 12h 43m  ", Style::default().fg(Color::Gray)),
            Span::styled("SESSION: ", Style::default().fg(Color::Gray)),
            Span::styled("0xAF42  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("STATUS: ", Style::default().fg(Color::Gray)),
            Span::styled("READY", Style::default().fg(Color::Green)),
        ])
    ];

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(chunks[0]);

    let header_left = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::BOTTOM))
        .alignment(Alignment::Left);
    f.render_widget(header_left, header_layout[0]);

    let header_right = Paragraph::new("[ RE-SYNC ]")
        .block(Block::default().borders(Borders::BOTTOM))
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(header_right, header_layout[1]);

    // Left Sidebar
    let left_container = Block::default().borders(Borders::RIGHT);
    let inner_left_area = left_container.inner(main_chunks[0]);
    f.render_widget(left_container, main_chunks[0]);

    let inner_left_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(4), Constraint::Min(0), Constraint::Length(3)]).split(inner_left_area);
    let title_para = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" ", Style::default().bg(Color::White)), 
            Span::styled(" ORB CLI", Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
        ]), 
        Line::from(Span::styled("Toolkit Marketplace", Style::default().fg(Color::Gray)))
    ]);
    f.render_widget(title_para, inner_left_chunks[0]);
    
    let menu_items = vec![
        ListItem::new("> 1:EXECUTE").style(Style::default().fg(Color::Cyan).bg(Color::DarkGray)),
        ListItem::new(""),
        ListItem::new("  2:SEARCH").style(Style::default().fg(Color::Gray)),
        ListItem::new(""),
        ListItem::new("  3:LOGS").style(Style::default().fg(Color::Gray)),
        ListItem::new(""),
        ListItem::new("  4:CONFIG").style(Style::default().fg(Color::Gray)),
        ListItem::new(""),
        ListItem::new("  5:HELP").style(Style::default().fg(Color::Gray)),
    ];
    let menu = List::new(menu_items);
    f.render_widget(menu, inner_left_chunks[1]);
    
    let prompt_btn = Paragraph::new("[ λ PROMPT ]")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Gray)));
    
    let btn_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).split(inner_left_chunks[2]);
    f.render_widget(prompt_btn, btn_layout[1]);

    // Center
    let center_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Bar
            Constraint::Percentage(50), // Featured Packs
            Constraint::Percentage(50), // Package Details
        ])
        .split(main_chunks[1]);

    // Search Bar
    let search_border_color = if app.search_mode { Color::Yellow } else { Color::DarkGray };
    let search_block = Block::default()
        .title(" SEARCH (Press '/' to enter, 'Esc' to exit) ")
        .borders(Borders::ALL)
        .style(Style::default().fg(search_border_color));
    
    let cursor = if app.search_mode { "█" } else { "" };
    let search_para = Paragraph::new(format!("> {}{}", app.search_query, cursor))
        .block(search_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(search_para, center_chunks[0]);

    // Featured Packs
    let total = app.tools.len();
    let current = if total == 0 { 0 } else { app.selected_index + 1 };
    let packs_title = format!(" FEATURED_PACKS - #{}/{} packages ", current, total);
    
    let packs_block = Block::default()
        .title(packs_title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::DarkGray));
    let packs_area = packs_block.inner(center_chunks[1]);
    f.render_widget(packs_block, center_chunks[1]);

    let grid_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(packs_area);

    let top_row = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(grid_layout[0]);
    let bottom_row = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(grid_layout[1]);

    let cell_areas = [top_row[0], top_row[1], bottom_row[0], bottom_row[1]];

    for i in 0..4 {
        let tool_idx = app.scroll_offset + i;
        if let Some(tool) = app.tools.get(tool_idx) {
            let is_selected = tool_idx == app.selected_index;
            render_card(f, cell_areas[i], tool, "0x???", "Found in PATH", 100, "Unknown", "None", is_selected);
        }
    }

    // Details summary
    let summary_block = Block::default().title("PACKAGE_DETAILS_SUMMARY").borders(Borders::ALL).style(Style::default().fg(Color::DarkGray));
    let table_area = summary_block.inner(center_chunks[2]);
    f.render_widget(summary_block, center_chunks[2]);

    let header_cells = ["IDENTIFIER", "STATUS", "BRANCH", "LAST_SYNC", "HASH"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).style(Style::default().bg(Color::Black)).height(2);

    let rows = vec![
        Row::new(vec![
            Cell::from("orb-io-stream").style(Style::default().fg(Color::Gray)),
            Cell::from("OK").style(Style::default().fg(Color::Green)),
            Cell::from("main").style(Style::default().fg(Color::Gray)),
            Cell::from("2023.10.12").style(Style::default().fg(Color::Gray)),
            Cell::from("f9a8...e2").style(Style::default().fg(Color::Gray)),
        ]),
        Row::new(vec![
            Cell::from("orb-ui-ratatui").style(Style::default().fg(Color::Gray)),
            Cell::from("OK").style(Style::default().fg(Color::Green)),
            Cell::from("stable").style(Style::default().fg(Color::Gray)),
            Cell::from("2023.10.11").style(Style::default().fg(Color::Gray)),
            Cell::from("3e12...b9").style(Style::default().fg(Color::Gray)),
        ]),
    ];

    let t = Table::new(rows, [Constraint::Percentage(25), Constraint::Percentage(15), Constraint::Percentage(15), Constraint::Percentage(25), Constraint::Percentage(20)])
        .header(header)
        .column_spacing(1);
    f.render_widget(t, table_area);

    // Footer
    let footer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);
    
    let footer_left = Paragraph::new("F1 Help  |  F10/q Quit  |  hjkl/Arrows Navigate  |  / Search  |  Enter Run").style(Style::default().fg(Color::Gray));
    f.render_widget(footer_left, footer_layout[0]);

    let footer_right = Paragraph::new("Docs   GitHub   v1.0.4   λ ready_")
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer_right, footer_layout[1]);
}

fn render_card(f: &mut ratatui::Frame, area: Rect, title: &str, id: &str, desc: &str, pop: u16, size: &str, reqs: &str, is_selected: bool) {
    let border_color = if is_selected { Color::Cyan } else { Color::DarkGray };
    let border_modifier = if is_selected { Modifier::BOLD } else { Modifier::empty() };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(border_color).add_modifier(border_modifier));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(2),
        Constraint::Length(2), // Desc
        Constraint::Length(2), // Popularity
        Constraint::Length(1), // Size and Reqs
    ]).split(inner);

    let title_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(70), Constraint::Percentage(30)]).split(chunks[0]);
    f.render_widget(Paragraph::new(Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))), title_layout[0]);
    f.render_widget(Paragraph::new(Span::styled(format!("ID: {}", id), Style::default().fg(Color::DarkGray))).alignment(Alignment::Right), title_layout[1]);

    f.render_widget(Paragraph::new(desc).style(Style::default().fg(Color::Gray)).wrap(Wrap { trim: true }), chunks[1]);

    let pop_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(12), Constraint::Min(0), Constraint::Length(5)]).split(chunks[2]);
    f.render_widget(Paragraph::new("POPULARITY:").style(Style::default().fg(Color::DarkGray)), pop_layout[0]);
    
    let width = pop_layout[1].width as usize;
    if width > 0 {
        let filled = (pop as usize * width) / 100;
        let empty = width.saturating_sub(filled);
        let bar = format!("{}{}", "█".repeat(filled), "▒".repeat(empty));
        f.render_widget(Paragraph::new(Span::styled(bar, Style::default().fg(Color::White))), pop_layout[1]);
    }
    
    f.render_widget(Paragraph::new(format!("{}%", pop)).alignment(Alignment::Right).style(Style::default().fg(Color::Gray)), pop_layout[2]);

    let bottom_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(40), Constraint::Percentage(60)]).split(chunks[3]);
    f.render_widget(Paragraph::new(format!("SIZE: {}", size)).style(Style::default().fg(Color::DarkGray)), bottom_layout[0]);
    f.render_widget(Paragraph::new(format!("REQS: {}", reqs)).style(Style::default().fg(Color::DarkGray)), bottom_layout[1]);
}

