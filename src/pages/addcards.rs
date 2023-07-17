

use std::io::Stdout;



use speki_backend::card::{Card, CardCache, SavedCard};
use speki_backend::categories::Category;



use speki_backend::Id;





use crossterm::cursor::{MoveDown};



use crossterm::{
    cursor::MoveTo,
    event::{KeyCode},
    execute,
    terminal::{Clear, ClearType},
};

use crate::pages::move_far_left;

use super::{choose_folder, draw_message, read_user_input, write_string};

pub fn add_card(
    stdout: &mut Stdout,
    category: &mut Category,
    cache: &mut CardCache,
) -> Option<SavedCard> {
    loop {
        execute!(stdout, Clear(ClearType::All)).unwrap();
        execute!(stdout, MoveTo(0, 0)).unwrap();
        let msg = format!("{}\n\t--front side--", category.print_full());

        write_string(stdout, &msg);
        execute!(stdout, MoveTo(0, 2)).unwrap();
        let mut key_code;

        let (front_text, code) = match read_user_input(stdout) {
            Some((text, code)) => (text, code),
            None => return None,
        };

        if code == KeyCode::Char('`') {
            if let Some(the_category) = choose_folder(stdout, "Choose new category") {
                *category = the_category;
            }
            continue;
        }

        key_code = code;

        let back_text = if key_code != KeyCode::Tab {
            execute!(stdout, MoveDown(2)).unwrap();
            move_far_left(stdout);
            println!("\t--back side--");
            move_far_left(stdout);

            let (back_text, code) = match read_user_input(stdout) {
                Some((text, code)) => (text, code),
                None => return None,
            };

            key_code = code;

            back_text
        } else {
            String::new()
        };

        let mut card = Card::new_simple(front_text, back_text);

        if key_code == KeyCode::Tab {
            card.meta.finished = false;
        }

        return Some(card.save_new_card(category, cache));
    }
}

pub fn add_the_cards(stdout: &mut Stdout, mut category: Category, cache: &mut CardCache) {
    loop {
        if add_card(stdout, &mut category, cache).is_none() {
            return;
        }
    }
}

pub fn add_dependency(
    stdout: &mut Stdout,
    card: &Id,
    category: Option<&Category>,
    cache: &mut CardCache,
) -> Option<SavedCard> {
    let mut card = cache.get_owned(card);
    let category = category.unwrap_or_else(|| card.category());
    let category = &mut category.to_owned();
    let new_dependency = add_card(stdout, category, cache)?;
    let info = card.set_dependency(new_dependency.id(), cache);

    if let Some(info) = info {
        draw_message(stdout, &info);
    }
    cache.refresh();
    Some(new_dependency)
}

pub fn add_dependent(
    stdout: &mut Stdout,
    card: &Id,
    category: Option<&Category>,
    cache: &mut CardCache,
) -> Option<SavedCard> {
    let mut card = cache.get_owned(card);
    let mut category = category.cloned().unwrap_or_else(|| card.category().clone());
    let new_dependent = add_card(stdout, &mut category, cache)?;
    let info = card.set_dependent(new_dependent.id(), cache);

    if let Some(info) = info {
        draw_message(stdout, &info);
    }
    cache.refresh();
    Some(new_dependent)
}
