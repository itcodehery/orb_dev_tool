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
use std::{error::Error, io, collections::HashSet, process::{Command, Stdio}, time::{Duration, Instant}};
use std::fs;
use rand::seq::SliceRandom;

#[derive(PartialEq)]
enum Screen {
    Marketplace,
    CreateFlowPrompt,
    FlowDiagram,
    FlowExecution,
}

#[derive(PartialEq)]
enum ExecutionStatus {
    WaitingForInput,
    Running,
    Completed,
}

struct App {
    all_tools: Vec<String>,
    tools: Vec<String>,
    search_query: String,
    search_mode: bool,
    selected_index: usize,
    scroll_offset: usize,

    screen: Screen,
    selected_for_flow: Vec<String>, // ordered selection
    prompt_input: String,
    flow_tools: Vec<String>,
    
    execution_input: String,
    execution_step: usize,
    execution_status: ExecutionStatus,
    last_step_time: Option<Instant>,
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

            screen: Screen::Marketplace,
            selected_for_flow: Vec::new(),
            prompt_input: String::new(),
            flow_tools: Vec::new(),
            
            execution_input: String::new(),
            execution_step: 0,
            execution_status: ExecutionStatus::WaitingForInput,
            last_step_time: None,
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

    fn toggle_selection(&mut self) {
        if let Some(tool) = self.tools.get(self.selected_index) {
            if let Some(pos) = self.selected_for_flow.iter().position(|x| x == tool) {
                self.selected_for_flow.remove(pos);
            } else {
                self.selected_for_flow.push(tool.clone());
            }
        }
    }

    fn create_manual_flow(&mut self) {
        if !self.selected_for_flow.is_empty() {
            self.flow_tools = self.selected_for_flow.clone();
            self.screen = Screen::FlowDiagram;
            self.selected_for_flow.clear();
        }
    }

    fn generate_ai_flow(&mut self) {
        // Mock AI response by picking 3 random tools
        let mut rng = rand::rng();
        let mut sample = self.all_tools.clone();
        sample.shuffle(&mut rng);
        self.flow_tools = sample.into_iter().take(3).collect();
        self.screen = Screen::FlowDiagram;
    }

    fn start_execution(&mut self) {
        self.execution_status = ExecutionStatus::Running;
        self.execution_step = 0;
        self.last_step_time = Some(Instant::now());
    }

    fn tick(&mut self) {
        if self.execution_status == ExecutionStatus::Running {
            if let Some(last_time) = self.last_step_time {
                if last_time.elapsed() >= Duration::from_secs(2) {
                    self.execution_step += 1;
                    if self.execution_step >= self.flow_tools.len() {
                        self.execution_status = ExecutionStatus::Completed;
                    } else {
                        self.last_step_time = Some(Instant::now());
                    }
                }
            }
        }
    }

    fn run_selected_standalone(&mut self) {
        if let Some(tool) = self.tools.get(self.selected_index) {
            let tool_clone = tool.clone();
            std::thread::spawn(move || {
                let mut cmd = Command::new(&tool_clone);
                cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
                let _ = cmd.spawn();
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
        app.tick();
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Global Navigation
                if !app.search_mode && app.screen != Screen::CreateFlowPrompt && (app.screen != Screen::FlowExecution || app.execution_status != ExecutionStatus::WaitingForInput) {
                    match key.code {
                        KeyCode::Char('1') => app.screen = Screen::Marketplace,
                        KeyCode::Char('2') => app.screen = Screen::CreateFlowPrompt,
                        KeyCode::Char('3') => {
                            if !app.flow_tools.is_empty() {
                                app.screen = Screen::FlowDiagram;
                            }
                        }
                        _ => {}
                    }
                }

                match app.screen {
                    Screen::Marketplace => {
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
                                KeyCode::Char(' ') => app.toggle_selection(),
                                KeyCode::Char('c') => app.create_manual_flow(),
                                KeyCode::Enter => app.run_selected_standalone(),
                                _ => {}
                            }
                        }
                    }
                    Screen::CreateFlowPrompt => {
                        match key.code {
                            KeyCode::Esc => app.screen = Screen::Marketplace,
                            KeyCode::Enter => app.generate_ai_flow(),
                            KeyCode::Backspace => { app.prompt_input.pop(); },
                            KeyCode::Char(c) => app.prompt_input.push(c),
                            _ => {}
                        }
                    }
                    Screen::FlowDiagram => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                            KeyCode::Enter => {
                                app.screen = Screen::FlowExecution;
                                app.execution_status = ExecutionStatus::WaitingForInput;
                                app.execution_input.clear();
                            }
                            _ => {}
                        }
                    }
                    Screen::FlowExecution => {
                        if app.execution_status == ExecutionStatus::WaitingForInput {
                            match key.code {
                                KeyCode::Esc => app.screen = Screen::FlowDiagram,
                                KeyCode::Enter => app.start_execution(),
                                KeyCode::Backspace => { app.execution_input.pop(); },
                                KeyCode::Char(c) => app.execution_input.push(c),
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                                _ => {}
                            }
                        }
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

    let header_right = Paragraph::new("[ ORCHESTRATOR ]")
        .block(Block::default().borders(Borders::BOTTOM))
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Cyan));
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
        Line::from(Span::styled("Toolkit Orchestrator", Style::default().fg(Color::Gray)))
    ]);
    f.render_widget(title_para, inner_left_chunks[0]);
    
    let menu_items = vec![
        ListItem::new(if app.screen == Screen::Marketplace { "> 1: MARKETPLACE" } else { "  1: MARKETPLACE" })
            .style(if app.screen == Screen::Marketplace { Style::default().fg(Color::Cyan).bg(Color::DarkGray) } else { Style::default().fg(Color::Gray) }),
        ListItem::new(""),
        ListItem::new(if app.screen == Screen::CreateFlowPrompt { "> 2: CREATE FLOW" } else { "  2: CREATE FLOW" })
            .style(if app.screen == Screen::CreateFlowPrompt { Style::default().fg(Color::Cyan).bg(Color::DarkGray) } else { Style::default().fg(Color::Gray) }),
        ListItem::new(""),
        ListItem::new(if app.screen == Screen::FlowDiagram || app.screen == Screen::FlowExecution { "> 3: ACTIVE FLOW" } else { "  3: ACTIVE FLOW" })
            .style(if app.screen == Screen::FlowDiagram || app.screen == Screen::FlowExecution { Style::default().fg(Color::Cyan).bg(Color::DarkGray) } else { Style::default().fg(Color::Gray) }),
        ListItem::new(""),
        ListItem::new("  4: CONFIG").style(Style::default().fg(Color::DarkGray)),
    ];
    let menu = List::new(menu_items);
    f.render_widget(menu, inner_left_chunks[1]);
    
    let prompt_btn = Paragraph::new("[ λ PROMPT ]")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Gray)));
    
    let btn_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).split(inner_left_chunks[2]);
    f.render_widget(prompt_btn, btn_layout[1]);

    // Content Area
    match app.screen {
        Screen::Marketplace => render_marketplace(f, app, main_chunks[1]),
        Screen::CreateFlowPrompt => render_flow_prompt(f, app, main_chunks[1]),
        Screen::FlowDiagram => render_flow_diagram(f, app, main_chunks[1]),
        Screen::FlowExecution => render_flow_execution(f, app, main_chunks[1]),
    }

    // Footer
    let footer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(chunks[2]);
    
    let footer_text = match app.screen {
        Screen::Marketplace => "F10/q Quit | hjkl/Arrows Navigate | Space Select | c Create Flow | / Search",
        Screen::CreateFlowPrompt => "Esc Cancel | Enter Generate Flow",
        Screen::FlowDiagram => "Esc Cancel | Enter Setup Execution",
        Screen::FlowExecution => if app.execution_status == ExecutionStatus::WaitingForInput { "Esc Back | Enter Start Flow" } else { "q Quit" },
    };

    let footer_left = Paragraph::new(footer_text).style(Style::default().fg(Color::Gray));
    f.render_widget(footer_left, footer_layout[0]);

    let footer_right = Paragraph::new("Docs   GitHub   v1.0.4   λ ready_")
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer_right, footer_layout[1]);
}

fn render_marketplace(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Bar
            Constraint::Percentage(50), // Featured Packs
            Constraint::Percentage(50), // Package Details
        ])
        .split(area);

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
    f.render_widget(search_para, chunks[0]);

    // Featured Packs
    let total = app.tools.len();
    let current = if total == 0 { 0 } else { app.selected_index + 1 };
    
    let mut selection_info = String::new();
    if !app.selected_for_flow.is_empty() {
        selection_info = format!(" | {} tools selected for manual flow (Press 'c' to build) ", app.selected_for_flow.len());
    }

    let packs_title = format!(" FEATURED_PACKS - #{}/{} packages{} ", current, total, selection_info);
    
    let packs_block = Block::default()
        .title(packs_title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::DarkGray));
    let packs_area = packs_block.inner(chunks[1]);
    f.render_widget(packs_block, chunks[1]);

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
            let is_marked = app.selected_for_flow.contains(tool);
            render_card(f, cell_areas[i], tool, "0x???", "Found in PATH", 100, "Unknown", "None", is_selected, is_marked);
        }
    }

    // Details summary
    let summary_block = Block::default().title("PACKAGE_DETAILS_SUMMARY").borders(Borders::ALL).style(Style::default().fg(Color::DarkGray));
    let table_area = summary_block.inner(chunks[2]);
    f.render_widget(summary_block, chunks[2]);

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
}

fn render_card(f: &mut ratatui::Frame, area: Rect, title: &str, id: &str, desc: &str, pop: u16, size: &str, reqs: &str, is_selected: bool, is_marked: bool) {
    let mut border_color = Color::DarkGray;
    let mut title_color = Color::White;
    let mut border_modifier = Modifier::empty();

    if is_marked {
        border_color = Color::Green;
        title_color = Color::Green;
    }
    if is_selected {
        border_color = Color::Cyan;
        border_modifier = Modifier::BOLD;
        if is_marked {
            title_color = Color::Cyan;
        }
    }
    
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
    f.render_widget(Paragraph::new(Span::styled(title, Style::default().fg(title_color).add_modifier(Modifier::BOLD))), title_layout[0]);
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

fn render_flow_prompt(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let block = Block::default().title(" AGENTIC FLOW ORCHESTRATOR ").borders(Borders::ALL).style(Style::default().fg(Color::Cyan));
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(0)
        ])
        .margin(2)
        .split(inner_area);

    let instructions = Paragraph::new(vec![
        Line::from(Span::styled("Describe the task you want to accomplish.", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("Our Agentic AI will smartly identify the best tools and string them together into a Flow.", Style::default().fg(Color::Gray))),
    ]);
    f.render_widget(instructions, chunks[0]);

    let input_block = Block::default().title(" Input Prompt ").borders(Borders::ALL).style(Style::default().fg(Color::Yellow));
    let input = Paragraph::new(format!("> {}█", app.prompt_input)).block(input_block).style(Style::default().fg(Color::White));
    f.render_widget(input, chunks[1]);
}

fn render_flow_diagram(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let block = Block::default().title(" ACTIVE FLOW DIAGRAM ").borders(Borders::ALL).style(Style::default().fg(Color::Cyan));
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  AI Orchestrated Pipeline:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))));
    lines.push(Line::from(""));

    let mut diagram_line = vec![Span::raw("  ")];
    for (i, tool) in app.flow_tools.iter().enumerate() {
        diagram_line.push(Span::styled(format!("[ {} ]", tool), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD).bg(Color::DarkGray)));
        if i < app.flow_tools.len() - 1 {
            diagram_line.push(Span::styled("  ━━━▶  ", Style::default().fg(Color::DarkGray)));
        }
    }
    
    lines.push(Line::from(diagram_line));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Press Enter to setup execution.", Style::default().fg(Color::Yellow))));

    let p = Paragraph::new(lines);
    f.render_widget(p, inner_area);
}

fn render_flow_execution(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let block = Block::default().title(" FLOW EXECUTION ").borders(Borders::ALL).style(Style::default().fg(Color::Cyan));
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Diagram
            Constraint::Length(3), // Input
            Constraint::Min(0)     // Status/Logs
        ])
        .margin(2)
        .split(inner_area);

    // Diagram with live status
    let mut diagram_line = vec![Span::raw("  ")];
    for (i, tool) in app.flow_tools.iter().enumerate() {
        let (color, bg) = if app.execution_status == ExecutionStatus::Completed {
            (Color::Black, Color::Green)
        } else if i < app.execution_step {
            (Color::Black, Color::Green)
        } else if i == app.execution_step && app.execution_status == ExecutionStatus::Running {
            (Color::Black, Color::Yellow)
        } else {
            (Color::White, Color::DarkGray)
        };

        diagram_line.push(Span::styled(format!("[ {} ]", tool), Style::default().fg(color).add_modifier(Modifier::BOLD).bg(bg)));
        if i < app.flow_tools.len() - 1 {
            diagram_line.push(Span::styled("  ━━━▶  ", Style::default().fg(Color::DarkGray)));
        }
    }

    let p = Paragraph::new(vec![Line::from(""), Line::from(diagram_line)]);
    f.render_widget(p, chunks[0]);

    if app.execution_status == ExecutionStatus::WaitingForInput {
        let input_block = Block::default().title(" Data Input (Optional) ").borders(Borders::ALL).style(Style::default().fg(Color::Yellow));
        let input = Paragraph::new(format!("> {}█", app.execution_input)).block(input_block).style(Style::default().fg(Color::White));
        f.render_widget(input, chunks[1]);
    } else {
        let status_text = if app.execution_status == ExecutionStatus::Running {
            format!("Running Step {}/{}...", app.execution_step + 1, app.flow_tools.len())
        } else {
            "Flow Completed Successfully.".to_string()
        };
        let status_para = Paragraph::new(Span::styled(status_text, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
        f.render_widget(status_para, chunks[2]);
    }
}

