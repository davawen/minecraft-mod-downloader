use std::{collections::HashMap, io::{self, Write}, time::{Duration, Instant}, fs::File};
use crossterm::{terminal::{enable_raw_mode, EnterAlternateScreen, disable_raw_mode, LeaveAlternateScreen}, execute, event::{EnableMouseCapture, DisableMouseCapture, Event, KeyCode, self, ModifierKeyCode}};
use serde_json::Value;
use tui::{backend::{CrosstermBackend, Backend}, Terminal, widgets::{Block, Borders, Paragraph, List, ListItem, ListState}, layout::{Rect, Layout, Direction, Constraint}, style::{Style, Color}};

fn main() -> Result<(), Box<dyn std::error::Error>> {

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    if ui(&mut terminal).is_err() {

    }
    
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

struct Project {
    id: String,
    title: String,
    description: String,
    downloads: i64
}

fn search_projects(query: &str) -> Result<Vec<Project>, reqwest::Error> {
    let res = reqwest::blocking::get(format!("https://api.modrinth.com/v2/search?query={query}&facets=[[\"categories:fabric\"], [\"project_type:mod\"]]"))?;
    let json = res.json::<HashMap<String, Value>>()?;

    let mut f = File::create("/tmp/out.txt").unwrap();
    f.write_all(&format!("{:#?}", json).into_bytes()).unwrap();

    let out: Vec<_> = if let Value::Array(hits) = &json["hits"] {
        hits.iter().map(|p| {
            Project {
                id: p["project_id"].as_str().unwrap().to_string(),
                title: p["title"].as_str().unwrap().to_string(),
                description: p["description"].as_str().unwrap().to_string(),
                downloads: p["downloads"].as_i64().unwrap()
            }
        }).collect()
    }
    else { vec![] };

    Ok(out)
}

#[derive(Debug, Clone, Copy)]
enum State {
    Normal,
    Insert
}

fn ui<B: Backend>(terminal: &mut Terminal<B>) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = State::Normal;
    let mut search = String::new();
    let mut results: Vec<Project> = vec![];
    let mut results_state = ListState::default();
    results_state.select(Some(0));
    let mut last_modification: Option<Instant> = Some(Instant::now());

    loop {
        terminal.draw(|f| {
            let main_frame = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(f.size());

            let mod_pane = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(main_frame[0]);

            let search_block = Block::default()
                .title("Search:")
                .borders(Borders::all())
                .style(
                    if let State::Insert = state { Style::default().fg(Color::Yellow) }
                    else { Style::default() }
                );

            let search = Paragraph::new(format!("{}{}", search, if let State::Insert = state { "|" } else { " " }))
                .block(search_block);
            
            let mods = List::new(results.iter().map(|p| {
                ListItem::new(p.title.clone())
            }).collect::<Vec<_>>())
                .block(Block::default().title("Mods:").borders(Borders::all()))
                .highlight_style( Style::default().fg(Color::Yellow) )
                .highlight_symbol("> ");

            let version = Paragraph::new("1.19.2")
                .block(Block::default().title("Version:").borders(Borders::all()));

            f.render_widget(search, mod_pane[0]);
            f.render_stateful_widget(mods, mod_pane[1], &mut results_state);
            f.render_widget(version, main_frame[1]);
        })?;

        if let Some(last_mod) = last_modification {
            if last_mod.elapsed() > Duration::from_millis(400) {
                results = search_projects(&search)?;
                last_modification = None;
            }
        }

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match state {
                    State::Normal => {
                        match key.code {
                            KeyCode::Char('q') => {
                                return Ok(());
                            },
                            KeyCode::Char('f') => {
                                state = State::Insert;
                            },
                            KeyCode::Char('j') => {
                                results_state.select(Some(results_state.selected().unwrap_or(0) + 1));
                            },
                            KeyCode::Char('k') => {
                                results_state.select(Some(results_state.selected().unwrap_or(0) - 1));
                            }

                            _ => {}
                        }
                    },
                    State::Insert => {
                        match key.code {
                            KeyCode::Char(c) => { 
                                search.push(c);
                                last_modification = Some(Instant::now());
                            },
                            KeyCode::Backspace => { search.pop(); },
                            KeyCode::Esc => state = State::Normal,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
