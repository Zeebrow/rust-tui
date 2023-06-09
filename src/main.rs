use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, size as ctsize, SetTitle, SetSize},
    execute,
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::{fs::{self, DirEntry}, path::PathBuf};
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};
use std::path::Path;
use std::env::current_dir;

const DB_PATH: &str = "./data/db.json";

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Serialize, Deserialize, Clone)]
struct Pet {
    id: usize,
    name: String,
    category: String,
    age: usize,
    created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Pets,
    Channels,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Pets => 1,
            MenuItem::Channels=> 2,
        }
    }
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cols, rows) = ctsize()?;
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec![
        "Home(F1)",
        "Channels(F2)",
        "Pets(F3)",
        "Quit(F4)"
    ];
    let pets_submenu_actions = vec![
        "Add",
        "Delete"
    ];
    let mut active_menu_item = MenuItem::Home;
    let mut pet_list_state = ListState::default();
    let mut chans_list_state = ListState::default();
    pet_list_state.select(Some(0));
    chans_list_state.select(Some(0));
    let mut cwd = current_dir().unwrap();
    // let cur1 = terminal.get_cursor().unwrap_or_else(|_e|(u16::MAX, u16::MAX)).0;
    // let cur2 = terminal.get_cursor().unwrap_or_else(|_e|(u16::MAX, u16::MAX)).1;
    // terminal.show_cursor().unwrap_or_else(|e| cur1 = e.to_string());
    // terminal.set_cursor(50, 50).unwrap_or_else(|e| cur1 = e.to_string());
    // NOTE: does not 'freeze' terminal
    let cur1 = String::from("N/A");
    let cur2 = String::from("N/A");
    // let c1: u16;
    // let c2: u16;
    // (c1, c2) = terminal.get_cursor().unwrap_or_else(|e| {
    //     cur1 = String::from("could not get cursor");
    //     cur2 = e.to_string();
    //     (u16::MAX, u16::MAX)
    // });
    // cur1 = c1.to_string();
    // cur2 = c2.to_string();

    execute!(std::io::stdout(), SetTitle("taken over by Rust"))?;
    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(10),
                    ]
                    .as_ref(),
                )
                .split(size);

            let copyright_text = std::format!("area: {} | top: {} | bottom: {} | left: {} | right: {} | cursor: {},{}", 
                size.area(), size.top(), size.bottom(), size.left(), size.right(), cur1, cur2);

            let copyright = Paragraph::new(copyright_text)
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title("Stats")
                        .border_type(BorderType::Plain),
                );

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);
            match active_menu_item {
                MenuItem::Home => rect.render_widget(render_home(), chunks[1]),
                MenuItem::Pets => {
                    let pets_menu_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [Constraint::Length(3), Constraint::Min(10)].as_ref(),
                        )
                        .split(chunks[1]);
                    //@@@
                    // y no work?@!
                    // let pm = Spans::from(pets_submenu_actions);
                    let pets_menu: Vec<Spans> = pets_submenu_actions.iter().map(|t|{
                        let (first, rest) = t.split_at(1);
                        Spans::from(vec![
                            Span::styled(first, Style::default().bg(Color::Green)),
                            Span::styled(rest, Style::default()),
                        ])
                    }).collect();
                    let pets_tabs = Tabs::new(pets_menu)
                        .block(Block::default().title("Pets - Actions"))
                        .divider(Span::raw(":"));
                    rect.render_widget(pets_tabs, pets_menu_chunks[0]);

                    let pets_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(pets_menu_chunks[1]);
                    let (left, right) = render_pets(&pet_list_state);
                    rect.render_stateful_widget(left, pets_chunks[0], &mut pet_list_state);
                    rect.render_widget(right, pets_chunks[1]);
                },
                MenuItem::Channels => {
                    let files_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref()
                        )
                        .split(chunks[1]);
                    rect.render_stateful_widget(render_files_list(&chans_list_state), files_chunks[0], &mut chans_list_state);
                    rect.render_widget(render_chans_contents(), files_chunks[1]);
                }
            }
            rect.render_widget(copyright, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::F(4) => {
                    /*quit*/
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::F(1) => active_menu_item = MenuItem::Home,
                KeyCode::F(2) => active_menu_item = MenuItem::Pets,
                KeyCode::F(3) => active_menu_item = MenuItem::Channels,
                KeyCode::Char('a') => {
                    match active_menu_item {
                        MenuItem::Channels => {
                            //add a channel to the sidebar and stuff
                        }
                        MenuItem::Pets => {
                            add_random_pet_to_db().expect("can add new random pet");
                        }
                        _ => {}
                    }
                    
                }
                KeyCode::Char('d') => {
                    match active_menu_item {
                        MenuItem::Pets => {
                            remove_pet_at_index(&mut pet_list_state).expect("can remove pet");
                        }
                        _ => {}
                    }
                }
                KeyCode::Down => {
                    match active_menu_item {
                        MenuItem::Pets => {
                            if let Some(selected) = pet_list_state.selected() {
                                let amount_pets = read_db().expect("can fetch pet list").len();
                                if selected >= amount_pets - 1 {
                                    pet_list_state.select(Some(0));
                                } else {
                                    pet_list_state.select(Some(selected + 1));
                                }
                            }
                        }
                        MenuItem::Channels => {
                            if let Some(selected) = chans_list_state.selected() {
                                let amount_files = get_chans_list().len();
                                if selected >= amount_files - 1 {
                                    chans_list_state.select(Some(0));
                                } else {
                                    chans_list_state.select(Some(selected + 1)); }
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Up => {
                    match active_menu_item {
                        MenuItem::Pets => {
                            if let Some(selected) = pet_list_state.selected() {
                                let amount_pets = read_db().expect("can fetch pet list").len();
                                if selected > 0 {
                                    pet_list_state.select(Some(selected - 1));
                                } else {
                                    pet_list_state.select(Some(amount_pets - 1));
                                }
                            }
                        }
                        MenuItem::Channels=> {
                            if let Some(selected) = chans_list_state.selected() {
                                let amount_files = get_chans_list().len();
                                if selected > 0 {
                                    chans_list_state.select(Some(selected - 1));
                                } else {
                                    chans_list_state.select(Some(amount_files - 1));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }
    // "Clean up when you're done" -the docs
    execute!(std::io::stdout(), SetSize(cols, rows))?;
    // reset terminal title
    execute!(std::io::stdout(), SetTitle(""))?;
    // Exit the application, and return to where you left off in the terminal
    // terminal.set_cursor(c1, c2).unwrap();
    // terminal.set_cursor(c1 + ctsize()?.0, c2 + ctsize()?.1).unwrap();
    // execute!(std::io::stdout(), Clear(ClearType::FromCursorDown))?;
    // execute!(std::io::stdout(), Print("k".to_string()))?;
    Ok(())
}


fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "pet-CLI",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 'p' to access pets, 'a' to add random new pets and 'd' to delete the currently selected pet.")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Home")
            .border_type(BorderType::Plain),
    );
    home
}

fn render_chans_contents<'a>() -> Paragraph<'a> {
    let files = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Channels:")]),
        Spans::from(vec![Span::styled(
            "~~todo~~",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Channels")
            .border_type(BorderType::Plain),
    );
    files
}

fn render_files_list<'a>(chans_list_state: &ListState) -> List<'a> {
    let files_list: Block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::DarkGray))
        .title("Your files")
        .border_type(BorderType::Double);

    let chan_names_list = vec!["strager", "het_tanis"];
    let items: Vec<_> = chan_names_list
        .iter()
        .map(|chan| {
            let name = chan.to_string();
            ListItem::new(Spans::from(
                vec![Span::styled(name, Style::default())]
            ))
        })
        .collect();

    let selected_file = chan_names_list
        .get(chans_list_state
            .selected()
            .expect("There is always '.' and '..' in any directory.")
        )
        .expect("exists..")
        .clone();

    let list = List::new(items).block(files_list).highlight_symbol(">> ");
    list

}

fn render_pets<'a>(pet_list_state: &ListState) -> (List<'a>, Table<'a>) {
    let pets = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Pets")
        .border_type(BorderType::Plain);

    let pet_list = read_db().expect("can fetch pet list");
    let items: Vec<_> = pet_list
        .iter()
        .map(|pet| {
            ListItem::new(Spans::from(vec![Span::styled(
                pet.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_pet = pet_list
        .get(
            pet_list_state
                .selected()
                .expect("there is always a selected pet"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(pets).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let pet_detail = Table::new(vec![Row::new(vec![
        Cell::from(Span::raw(selected_pet.id.to_string())),
        Cell::from(Span::raw(selected_pet.name)),
        Cell::from(Span::raw(selected_pet.category)),
        Cell::from(Span::raw(selected_pet.age.to_string())),
        Cell::from(Span::raw(selected_pet.created_at.to_string())),
    ])])
    .header(Row::new(vec![
        Cell::from(Span::styled(
            "ID",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Name",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Category",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Age",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Created At",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Detail")
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(5),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(5),
        Constraint::Percentage(20),
    ]);

    (list, pet_detail)
}

fn read_db() -> Result<Vec<Pet>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<Pet> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn add_random_pet_to_db() -> Result<Vec<Pet>, Error> {
    let mut rng = rand::thread_rng();
    let db_content = fs::read_to_string(DB_PATH)?;
    let mut parsed: Vec<Pet> = serde_json::from_str(&db_content)?;
    let catsdogs = match rng.gen_range(0, 1) {
        0 => "cats",
        _ => "dogs",
    };

    let random_pet = Pet {
        id: rng.gen_range(0, 9999999),
        name: rng.sample_iter(Alphanumeric).take(10).collect(),
        category: catsdogs.to_owned(),
        age: rng.gen_range(1, 15),
        created_at: Utc::now(),
    };

    parsed.push(random_pet);
    fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
    Ok(parsed)
}

// need state
fn remove_channel(chans_list_state: &mut ListState) -> Result<(), Error> {
    let selection = chans_list_state.selected();
    if let Some(selected) = chans_list_state.selected() {
        println!("{}", Some(selection));
        //bot.remove()
    }
    Ok(())
}

fn remove_pet_at_index(pet_list_state: &mut ListState) -> Result<(), Error> {
    if let Some(selected) = pet_list_state.selected() {
        let db_content = fs::read_to_string(DB_PATH)?;
        let mut parsed: Vec<Pet> = serde_json::from_str(&db_content)?;
        parsed.remove(selected);
        fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
        let amount_pets = read_db().expect("can fetch pet list").len();
        if selected > 0 {
            pet_list_state.select(Some(selected - 1));
        } else {
            pet_list_state.select(Some(0));
        }
    }
    Ok(())
}

fn get_chans_list() -> Vec<String> {
    vec![String::from("strager"), String::from("het_tanis")]
}

// fn get_files_list(dir: std::path::PathBuf) -> Vec<DirEntry> {
//     // let db_content = fs::read_to_string(DB_PATH)?;
//     // let parsed: Vec<Pet> = serde_json::from_str(&db_content)?;
//     // Ok(parsed)
//     /*
//     let files_list = dir.read_dir().expect("read_dir call failed").unwrap_or_else(|| ).map(|f| {
//         if let Ok(f) = f{
//             f;
//         }
//     }).collect();
//     */
//     // let files_list: Vec<FSFile> = vec![
//     //     FSFile{ name: String::from("file1.txt") },
//     //     FSFile{ name: String::from("file2.txt") },
//     //     FSFile{ name: String::from("file3.txt") },
//     // ];

// }