use std::collections::{BTreeSet, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use speki_backend::card::{Card, CardCache, IsSuspended, Priority, ReviewType, Reviews, SavedCard};
use speki_backend::categories::Category;
use speki_backend::common::duration_to_days;
use speki_backend::common::{current_time, truncate_string};

use speki_backend::Id;

use ascii_tree::write_tree;

use rand::seq::SliceRandom;

use crossterm::cursor::{self, MoveDown, MoveLeft};
use crossterm::event::KeyEvent;
use crossterm::style::Print;
use crossterm::terminal;
use crossterm::{
    cursor::MoveTo,
    event::{read, Event, KeyCode},
    execute,
    style::{ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

pub fn view_cards(stdout: &mut Stdout, mut cards: Vec<Id>, cache: &mut CardCache) {
    if cards.is_empty() {
        draw_message(stdout, "No cards found");
        return;
    }

    let mut selected = 0;

    loop {
        let card_qty = cards.len();
        let card = cache.get_ref(&cards[selected]);
        let mut excluded_cards = HashSet::new();
        excluded_cards.insert(card.id().to_owned());

        let message = format!(
            "{}/{}\t{}\n{}\n-------------------\n{}",
            selected + 1,
            card_qty,
            card.category().print_full(),
            card.front_text(),
            card.back_text()
        );

        let key_event = draw_key_event_message(stdout, &message);

        if edit_card(stdout, &key_event.code, card.clone(), cache) {
            continue;
        }

        match key_event.code {
            KeyCode::Char('l') | KeyCode::Right if selected != card_qty - 1 => selected += 1,
            KeyCode::Char('h') | KeyCode::Left if selected != 0 => selected -= 1,
            KeyCode::Char('.') => panic!(),
            KeyCode::Char('X') => {
                if let Some(thecard) = ascii_test(stdout, card.id(), cache, true) {
                    let mut idx = None;
                    for card in cards.iter().enumerate() {
                        if card.1 == thecard.id() {
                            idx = Some(card.0);
                        }
                    }

                    if let Some(idx) = idx {
                        cards.swap(0, idx);
                        selected = 0;
                    } else {
                        draw_message(stdout, "damn ...");
                    }
                }
            }

            KeyCode::Char('x') => {
                if let Some(thecard) = ascii_test(stdout, card.id(), cache, false) {
                    let mut idx = None;
                    for card in cards.iter().enumerate() {
                        if card.1 == thecard.id() {
                            idx = Some(card.0);
                        }
                    }

                    if let Some(idx) = idx {
                        cards.swap(0, idx);
                        selected = 0;
                    } else {
                        draw_message(stdout, "damn ...");
                    }
                }
            }

            KeyCode::Char('T') => {
                draw_message(stdout, "Adding new dependent");
                let category = Some(card.category().to_owned());
                if let Some(updated_card) =
                    add_dependent(stdout, card.id(), category.as_ref(), cache)
                {
                    cards.insert(0, *updated_card.id());
                }
            }
            KeyCode::Char('Y') => {
                draw_message(stdout, "Adding new dependency");
                let category = Some(card.category().to_owned());
                if let Some(updated_card) =
                    add_dependency(stdout, card.id(), category.as_ref(), cache)
                {
                    cards.insert(0, *updated_card.id());
                }
            }

            KeyCode::Char('a') => {
                if let Some(card) = add_card(stdout, &mut card.category().clone(), cache) {
                    cards.insert(0, card.id().to_owned()); // temp thing
                }
            }
            KeyCode::Char('r') => {
                cards.remove(selected);
                if cards.is_empty() {
                    draw_message(stdout, "No more cards");
                    return;
                }
                if selected == cards.len() {
                    selected -= 1;
                }
            }
            KeyCode::Char('D') => {
                cache.get_owned(card.id()).delete(cache);
                draw_message(stdout, "Card deleted");
                cards.remove(selected);
                if cards.is_empty() {
                    draw_message(stdout, "No more cards");
                    return;
                }
                if selected == cards.len() {
                    selected -= 1;
                }
            }
            KeyCode::Char('s') => {}
            KeyCode::Char('/') => {
                if let Some(thecard) = search_for_item(stdout, "find some card", excluded_cards) {
                    let mut idx = None;
                    for card in cards.iter().enumerate() {
                        if card.1 == thecard.id() {
                            idx = Some(card.0);
                        }
                    }

                    if let Some(idx) = idx {
                        cards.swap(0, idx);
                        selected = 0;
                    } else {
                        draw_message(stdout, "damn ...");
                    }
                }
            }
            key if should_exit(&key) => return,
            _ => {}
        };
    }
}

pub fn view_all_cards(stdout: &mut Stdout, cache: &mut CardCache) {
    let cards = cache.all_ids();
    view_cards(stdout, cards, cache);
}
