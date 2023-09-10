use std::io::Stdout;

use speki_backend::card::{Card, CardCache, SavedCard};
use speki_backend::categories::Category;

use speki_backend::Id;





use crate::backend::get_text_from_vim;


use super::{draw_message};

pub fn add_card(category: &mut Category, cache: &mut CardCache) -> Option<SavedCard> {
    let text = get_text_from_vim(None).ok()??;

    let (front, back) = match text.split_once('\n') {
        Some((front, back)) => (front.trim().to_string(), back.trim().to_string()),
        None => (text, String::default()),
    };

    let is_finished = !back.is_empty();
    let mut card = Card::new_simple(front, back);
    card.meta.finished = is_finished;

    Some(card.save_new_card(category, cache))
}

pub fn add_the_cards(_stdout: &mut Stdout, mut category: Category, cache: &mut CardCache) {
    loop {
        if add_card(&mut category, cache).is_none() {
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
    let new_dependency = add_card(category, cache)?;
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
    let new_dependent = add_card(&mut category, cache)?;
    let info = card.set_dependent(new_dependent.id(), cache);

    if let Some(info) = info {
        draw_message(stdout, &info);
    }
    cache.refresh();
    Some(new_dependent)
}
