//! Rune TUI (Terminal User Interface)
//!
//! This module provides a terminal-based user interface for managing
//! containers, images, networks, and volumes.

use crate::container::{ContainerConfig, ContainerManager, ContainerStatus};
use crate::error::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Row, Table, TableState, Clear},
};
use std::io;
use std::sync::Arc;

/// TUI application state
pub struct App {
    /// Container manager
    container_manager: Arc<ContainerManager>,
    /// Current tab index
    current_tab: usize,
    /// Tab titles
    tabs: Vec<&'static str>,
    /// Container list state
    container_state: TableState,
    /// Image list state
    image_state: TableState,
    /// Network list state
    network_state: TableState,
    /// Volume list state
    volume_state: TableState,
    /// Should quit
    should_quit: bool,
    /// Show help
    show_help: bool,
    /// Status message
    status_message: Option<String>,
    /// Containers cache
    containers: Vec<ContainerConfig>,
}

impl App {
    /// Create a new TUI application
    pub fn new(container_manager: Arc<ContainerManager>) -> Self {
        Self {
            container_manager,
            current_tab: 0,
            tabs: vec!["Containers", "Images", "Networks", "Volumes", "Swarm"],
            container_state: TableState::default(),
            image_state: TableState::default(),
            network_state: TableState::default(),
            volume_state: TableState::default(),
            should_quit: false,
            show_help: false,
            status_message: None,
            containers: Vec::new(),
        }
    }

    /// Run the TUI application
    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main loop
        let result = self.main_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Main event loop
    fn main_loop<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Refresh data
            self.refresh_data()?;

            // Draw UI
            terminal.draw(|f| self.ui(f))?;

            // Handle events
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code)?;
                    }
                }
            }

            if self.should_quit {
                return Ok(());
            }
        }
    }

    /// Refresh data from managers
    fn refresh_data(&mut self) -> Result<()> {
        self.containers = self.container_manager.list(true)?;
        Ok(())
    }

    /// Handle key press
    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        if self.show_help {
            self.show_help = false;
            return Ok(());
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') | KeyCode::F(1) => self.show_help = true,
            KeyCode::Tab | KeyCode::Right => {
                self.current_tab = (self.current_tab + 1) % self.tabs.len();
            }
            KeyCode::BackTab | KeyCode::Left => {
                if self.current_tab == 0 {
                    self.current_tab = self.tabs.len() - 1;
                } else {
                    self.current_tab -= 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => self.handle_enter()?,
            KeyCode::Char('s') => self.handle_start()?,
            KeyCode::Char('S') => self.handle_stop()?,
            KeyCode::Char('r') => self.handle_restart()?,
            KeyCode::Char('d') | KeyCode::Delete => self.handle_delete()?,
            KeyCode::Char('p') => self.handle_pause()?,
            KeyCode::Char('u') => self.handle_unpause()?,
            _ => {}
        }

        Ok(())
    }

    /// Select previous item
    fn select_previous(&mut self) {
        let state = match self.current_tab {
            0 => &mut self.container_state,
            1 => &mut self.image_state,
            2 => &mut self.network_state,
            3 => &mut self.volume_state,
            _ => return,
        };

        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    /// Select next item
    fn select_next(&mut self) {
        let (state, len) = match self.current_tab {
            0 => (&mut self.container_state, self.containers.len()),
            1 => (&mut self.image_state, 0), // TODO: Get image count
            2 => (&mut self.network_state, 0), // TODO: Get network count
            3 => (&mut self.volume_state, 0), // TODO: Get volume count
            _ => return,
        };

        if len == 0 {
            return;
        }

        let i = match state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    len - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    /// Handle enter key
    fn handle_enter(&mut self) -> Result<()> {
        self.status_message = Some("Enter pressed - would show details".to_string());
        Ok(())
    }

    /// Handle start action
    fn handle_start(&mut self) -> Result<()> {
        if self.current_tab == 0 {
            if let Some(i) = self.container_state.selected() {
                if let Some(container) = self.containers.get(i) {
                    match self.container_manager.start(&container.id) {
                        Ok(_) => {
                            self.status_message = Some(format!("Started container {}", container.name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle stop action
    fn handle_stop(&mut self) -> Result<()> {
        if self.current_tab == 0 {
            if let Some(i) = self.container_state.selected() {
                if let Some(container) = self.containers.get(i) {
                    match self.container_manager.stop(&container.id) {
                        Ok(_) => {
                            self.status_message = Some(format!("Stopped container {}", container.name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle restart action
    fn handle_restart(&mut self) -> Result<()> {
        if self.current_tab == 0 {
            if let Some(i) = self.container_state.selected() {
                if let Some(container) = self.containers.get(i) {
                    let _ = self.container_manager.stop(&container.id);
                    match self.container_manager.start(&container.id) {
                        Ok(_) => {
                            self.status_message = Some(format!("Restarted container {}", container.name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle delete action
    fn handle_delete(&mut self) -> Result<()> {
        if self.current_tab == 0 {
            if let Some(i) = self.container_state.selected() {
                if let Some(container) = self.containers.get(i) {
                    match self.container_manager.remove(&container.id, true) {
                        Ok(_) => {
                            self.status_message = Some(format!("Removed container {}", container.name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle pause action
    fn handle_pause(&mut self) -> Result<()> {
        if self.current_tab == 0 {
            if let Some(i) = self.container_state.selected() {
                if let Some(container) = self.containers.get(i) {
                    match self.container_manager.pause(&container.id) {
                        Ok(_) => {
                            self.status_message = Some(format!("Paused container {}", container.name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle unpause action
    fn handle_unpause(&mut self) -> Result<()> {
        if self.current_tab == 0 {
            if let Some(i) = self.container_state.selected() {
                if let Some(container) = self.containers.get(i) {
                    match self.container_manager.unpause(&container.id) {
                        Ok(_) => {
                            self.status_message = Some(format!("Unpaused container {}", container.name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Render UI
    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(3),  // Tabs
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Status bar
            ])
            .split(f.area());

        // Header
        self.render_header(f, chunks[0]);

        // Tabs
        self.render_tabs(f, chunks[1]);

        // Content
        match self.current_tab {
            0 => self.render_containers(f, chunks[2]),
            1 => self.render_images(f, chunks[2]),
            2 => self.render_networks(f, chunks[2]),
            3 => self.render_volumes(f, chunks[2]),
            4 => self.render_swarm(f, chunks[2]),
            _ => {}
        }

        // Status bar
        self.render_status_bar(f, chunks[3]);

        // Help overlay
        if self.show_help {
            self.render_help(f);
        }
    }

    /// Render header
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("🔮 Rune", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Docker-compatible Container Service"),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)));
        f.render_widget(title, area);
    }

    /// Render tabs
    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let titles: Vec<Line> = self.tabs.iter().map(|t| Line::from(*t)).collect();
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Navigation"))
            .select(self.current_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(tabs, area);
    }

    /// Render containers tab
    fn render_containers(&mut self, f: &mut Frame, area: Rect) {
        let header = Row::new(vec!["ID", "Name", "Image", "Status", "Created"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = self.containers.iter().map(|c| {
            let status_color = match c.status {
                ContainerStatus::Running => Color::Green,
                ContainerStatus::Paused => Color::Yellow,
                ContainerStatus::Stopped | ContainerStatus::Exited => Color::Red,
                _ => Color::Gray,
            };

            Row::new(vec![
                c.id[..12].to_string(),
                c.name.clone(),
                c.image.clone(),
                format!("{}", c.status),
                c.created_at.format("%Y-%m-%d %H:%M").to_string(),
            ])
            .style(Style::default().fg(Color::White))
            .height(1)
        }).collect();

        let widths = [
            Constraint::Length(14),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Length(12),
            Constraint::Length(18),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Containers"))
            .row_highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(table, area, &mut self.container_state);
    }

    /// Render images tab
    fn render_images(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Images");

        let text = Paragraph::new("No images found. Pull or build images to see them here.")
            .block(block)
            .style(Style::default().fg(Color::Gray));

        f.render_widget(text, area);
    }

    /// Render networks tab
    fn render_networks(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Networks");

        let text = Paragraph::new("Default networks:\n  • bridge\n  • host\n  • none")
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(text, area);
    }

    /// Render volumes tab
    fn render_volumes(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Volumes");

        let text = Paragraph::new("No volumes found. Create volumes to see them here.")
            .block(block)
            .style(Style::default().fg(Color::Gray));

        f.render_widget(text, area);
    }

    /// Render swarm tab
    fn render_swarm(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Swarm");

        let text = Paragraph::new("Swarm mode is not active.\n\nInitialize swarm with: rune swarm init")
            .block(block)
            .style(Style::default().fg(Color::Gray));

        f.render_widget(text, area);
    }

    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status = if let Some(ref msg) = self.status_message {
            msg.clone()
        } else {
            format!(
                "Containers: {} | Tab/←→: Switch tabs | ↑↓/jk: Navigate | ?: Help | q: Quit",
                self.containers.len()
            )
        };

        let status_bar = Paragraph::new(status)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)))
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(status_bar, area);
    }

    /// Render help overlay
    fn render_help(&self, f: &mut Frame) {
        let area = centered_rect(60, 70, f.area());

        f.render_widget(Clear, area);

        let help_text = vec![
            Line::from(Span::styled("Keyboard Shortcuts", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tab / ← →", Style::default().fg(Color::Cyan)),
                Span::raw("  Switch tabs"),
            ]),
            Line::from(vec![
                Span::styled("↑ ↓ / j k", Style::default().fg(Color::Cyan)),
                Span::raw("  Navigate list"),
            ]),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Cyan)),
                Span::raw("      View details"),
            ]),
            Line::from(vec![
                Span::styled("s", Style::default().fg(Color::Cyan)),
                Span::raw("          Start container"),
            ]),
            Line::from(vec![
                Span::styled("S", Style::default().fg(Color::Cyan)),
                Span::raw("          Stop container"),
            ]),
            Line::from(vec![
                Span::styled("r", Style::default().fg(Color::Cyan)),
                Span::raw("          Restart container"),
            ]),
            Line::from(vec![
                Span::styled("p", Style::default().fg(Color::Cyan)),
                Span::raw("          Pause container"),
            ]),
            Line::from(vec![
                Span::styled("u", Style::default().fg(Color::Cyan)),
                Span::raw("          Unpause container"),
            ]),
            Line::from(vec![
                Span::styled("d / Del", Style::default().fg(Color::Cyan)),
                Span::raw("    Delete container"),
            ]),
            Line::from(vec![
                Span::styled("? / F1", Style::default().fg(Color::Cyan)),
                Span::raw("     Show this help"),
            ]),
            Line::from(vec![
                Span::styled("q", Style::default().fg(Color::Cyan)),
                Span::raw("          Quit"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Press any key to close", Style::default().fg(Color::Gray))),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(Style::default().fg(Color::Yellow)))
            .alignment(Alignment::Left);

        f.render_widget(help, area);
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
