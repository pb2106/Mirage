use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mirage_plugin_host::{PluginHost, LoadedPlugin};
use mirage_protocol::{load_profile, Profile};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{fs, io, path::PathBuf};

struct App {
    profiles: Vec<PathBuf>,
    profile_list_state: ListState,
    loaded_profiles: Vec<Profile>,
    plugin_host: PluginHost,
}

impl App {
    fn new() -> Result<Self> {
        let mut app = Self {
            profiles: Vec::new(),
            profile_list_state: ListState::default(),
            loaded_profiles: Vec::new(),
            plugin_host: PluginHost::new(),
        };
        app.load_profiles()?;
        app.plugin_host.load_from_default_dir();
        if !app.profiles.is_empty() {
            app.profile_list_state.select(Some(0));
        }
        Ok(app)
    }

    fn load_profiles(&mut self) -> Result<()> {
        let profiles_dir = PathBuf::from("profiles");
        if !profiles_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(profiles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                self.profiles.push(path.clone());
                if let Ok(profile) = load_profile(&path) {
                    self.loaded_profiles.push(profile);
                }
            }
        }
        Ok(())
    }

    fn next_profile(&mut self) {
        let i = match self.profile_list_state.selected() {
            Some(i) => {
                if i >= self.loaded_profiles.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.profile_list_state.select(Some(i));
    }

    fn previous_profile(&mut self) {
        let i = match self.profile_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.loaded_profiles.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.profile_list_state.select(Some(i));
    }
}

fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down | KeyCode::Char('j') => app.next_profile(),
                KeyCode::Up | KeyCode::Char('k') => app.previous_profile(),
                _ => {}
            }
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" Mirage Identity Platform ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("— Press 'q' to quit"),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Mirage TUI "));
    f.render_widget(title, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(chunks[1]);

    // Profiles List
    let profiles: Vec<ListItem> = app
        .loaded_profiles
        .iter()
        .map(|p| {
            ListItem::new(Line::from(vec![Span::raw(p.name.clone())]))
        })
        .collect();

    let profiles_list = List::new(profiles)
        .block(Block::default().borders(Borders::ALL).title(" Profiles "))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(profiles_list, body_chunks[0], &mut app.profile_list_state);

    // Right pane: Profile details + Plugins
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(body_chunks[1]);

    // Profile Details
    let details_text = if let Some(i) = app.profile_list_state.selected() {
        if let Some(profile) = app.loaded_profiles.get(i) {
            let mut lines = vec![
                Line::from(vec![Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&profile.name)]),
                Line::from(vec![Span::styled("Hostname: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(profile.hostname.as_deref().unwrap_or("N/A"))]),
                Line::from(vec![Span::styled("Machine ID: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(profile.machine_id.as_deref().unwrap_or("N/A"))]),
                Line::from(vec![Span::styled("Timezone: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(profile.timezone.as_deref().unwrap_or("N/A"))]),
                Line::from(vec![Span::styled("Locale: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(profile.locale.as_deref().unwrap_or("N/A"))]),
                Line::from(vec![Span::styled("CPU Model: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(profile.cpu_model.as_deref().unwrap_or("N/A"))]),
                Line::from(vec![Span::styled("MAC Address: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(profile.mac_address.as_deref().unwrap_or("N/A"))]),
                Line::from(vec![Span::styled("Isolate Network: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(if profile.isolate_network.unwrap_or(false) { "Yes" } else { "No" })]),
            ];

            if let Some(gps) = &profile.gps {
                lines.push(Line::from(vec![Span::styled("GPS: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(format!("{}, {}", gps.lat, gps.lon))]));
            }

            if let Some(dns) = &profile.dns {
                lines.push(Line::from(vec![Span::styled("DNS: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(dns.join(", "))]));
            }

            lines
        } else {
            vec![Line::from("Select a profile")]
        }
    } else {
        vec![Line::from("No profile selected")]
    };

    let details_p = Paragraph::new(details_text)
        .block(Block::default().borders(Borders::ALL).title(" Profile Details "));
    f.render_widget(details_p, right_chunks[0]);

    // Plugins list
    let plugins_text = if app.plugin_host.is_empty() {
        vec![Line::from(Span::styled("No plugins loaded (check ~/.config/mirage/plugins/)", Style::default().fg(Color::DarkGray)))]
    } else {
        app.plugin_host.plugins().iter().map(|p| {
            Line::from(vec![
                Span::styled(format!("{} ", p.name), Style::default().fg(Color::Green)),
                Span::raw(format!("- {}", p.description)),
            ])
        }).collect()
    };

    let plugins_p = Paragraph::new(plugins_text)
        .block(Block::default().borders(Borders::ALL).title(" Loaded Plugins "));
    f.render_widget(plugins_p, right_chunks[1]);
}
