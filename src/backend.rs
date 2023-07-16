use std::collections::{BTreeSet, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use speki_backend::card::{Card, CardCache, IsSuspended, Priority, ReviewType, Reviews, SavedCard};
use speki_backend::categories::Category;
use speki_backend::common::{current_time, open_file_with_vim, randvec, truncate_string};
use speki_backend::common::{duration_to_days, view_cards_in_explorer};
use speki_backend::config::Config;
use speki_backend::git::git_save;
use speki_backend::ml::{
    five_review_stuff, fourplus_review_stuff, six_review_stuff, three_review_stuff,
    two_review_stuff,
};
use speki_backend::paths::get_share_path;
use speki_backend::Id;

use ascii_tree::write_tree;
use ascii_tree::Tree::Node;

use rand::seq::SliceRandom;

use crossterm::cursor::{self, MoveDown, MoveLeft, Show};
use crossterm::event::KeyEvent;
use crossterm::style::Print;
use crossterm::terminal;
use crossterm::{
    cursor::{Hide, MoveTo},
    event::{read, Event, KeyCode},
    execute,
    style::{ResetColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

pub fn to_ascii_tree(
    id: &Id,
    cache: &mut CardCache,
    show_dependencies: bool,
    visited: &mut BTreeSet<Id>,
) -> ascii_tree::Tree {
    visited.insert(*id);

    let card = cache.get_ref(id);
    let mut children = Vec::new();
    let dependencies = if show_dependencies {
        cache.recursive_dependencies(card.id())
    } else {
        cache.recursive_dependents(card.id())
    };

    for dependency in dependencies {
        if !visited.contains(&dependency) {
            children.push(to_ascii_tree(
                &dependency,
                cache,
                show_dependencies,
                visited,
            ));
        }
    }

    visited.remove(id);

    Node(card.front_text().to_owned(), children)
}

pub fn cards_as_string(cards: &Vec<SavedCard>) -> String {
    let mut s = String::new();

    for card in cards {
        s.push_str(card.front_text());
        s.push('\n');
    }
    s
}

pub fn import_stuff(cache: &mut CardCache) {
    let import_path = get_share_path().join("forimport.txt");
    if !import_path.exists() {
        return;
    }
    let category = Category::import_category();
    let cards = Card::import_cards(import_path.as_path());

    if let Some(cards) = cards {
        for card in cards {
            card.save_new_card(&category, cache);
        }
    }
    let to_path = get_share_path().join("imported.txt");
    std::fs::rename(import_path, to_path).unwrap();
}

pub fn should_exit(key: &KeyCode) -> bool {
    matches!(key, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q'))
}

pub fn get_following_unfinished_cards(category: &Category, cache: &mut CardCache) -> Vec<Id> {
    let categories = category.get_following_categories();
    let mut cards = vec![];
    for category in categories {
        cards.extend(category.get_unfinished_cards(cache));
    }
    randvec(cards)
}

pub type CardsFromCategory = Box<dyn FnMut(&Category, &mut CardCache) -> Vec<Id>>;
pub type SortCards = Box<dyn FnMut(&mut Vec<&SavedCard>, &mut CardCache)>;

pub fn get_keycode() -> KeyCode {
    loop {
        match read().unwrap() {
            Event::Key(KeyEvent { code, .. }) => return code,
            _ => continue,
        }
    }
}

pub fn get_key_event() -> KeyEvent {
    loop {
        match read().unwrap() {
            Event::Key(event) => return event,
            _ => continue,
        }
    }
}

pub fn _get_char() -> char {
    loop {
        if let Ok(Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            ..
        })) = read()
        {
            return c;
        }
    }
}
