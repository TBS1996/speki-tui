use std::collections::BTreeSet;

use speki_backend::card::{Card, CardCache, SavedCard};
use speki_backend::categories::Category;
use speki_backend::common::randvec;

use speki_backend::paths::get_share_path;
use speki_backend::Id;

use ascii_tree::Tree::Node;

use crossterm::event::KeyEvent;

use crossterm::event::{read, Event, KeyCode};

pub fn get_text_from_vim(
    initial_text: Option<String>,
    vim_rc: Option<&str>,
) -> std::io::Result<Option<String>> {
    use std::io::Read;
    use std::path::Path;
    use std::process::Command;

    let temp_file_path = "first line is front, second line is back";
    let vimrc_temp_path = "flashcard_vimrc.vim";

    if let Some(text) = initial_text {
        std::fs::write(temp_file_path, text)?;
    }

    let mut command = Command::new("nvim");

    if let Some(vim_rc) = vim_rc {
        std::fs::write(vimrc_temp_path, vim_rc)?;
        command.arg("-u").arg(vimrc_temp_path);
    }

    command
        .arg("-c")
        .arg("startinsert")
        .arg(temp_file_path)
        .status()?;

    // Cleanup the temporary vimrc file after Vim exits
    if vim_rc.is_some() {
        std::fs::remove_file(vimrc_temp_path)?;
    }

    Ok(Path::new(temp_file_path).exists().then_some({
        let mut file = std::fs::File::open(temp_file_path)?;
        let mut str_contents = String::new();
        file.read_to_string(&mut str_contents)?;
        std::fs::remove_file(temp_file_path)?;
        str_contents
    }))
}

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
//pub type SortCards = Box<dyn FnMut(&mut Vec<&SavedCard>, &mut CardCache)>;

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
