use std::collections::{BTreeSet, HashSet};
use std::fmt::Display;
use std::io::Stdout;
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

use crate::backend::{get_keycode, should_exit, CardsFromCategory};

use super::addcards::{add_card, add_dependency, add_dependent};
use super::viewcards::view_all_cards;
use super::{
    affirmative, ascii_test, draw_message, edit_card, print_card_review_back,
    print_card_review_front, update_status_bar, SomeStatus,
};

pub fn review_cards(
    stdout: &mut Stdout,
    category: Category,
    mut get_cards: CardsFromCategory,
    cache: &mut CardCache,
    toggle_refresh: bool,
    randomized: bool,
) {
    loop {
        let categories = category.get_following_categories();
        let mut cards = BTreeSet::new();
        for category in &categories {
            cards.extend(get_cards(category, cache));
        }

        let mut cards: Vec<Id> = cards.into_iter().collect();

        let cardqty = cards.len();

        if randomized {
            let mut rng = rand::thread_rng();
            cards.shuffle(&mut rng);
        } else {
            cards.sort_by_key(|card| {
                (cache.get_ref(card).expected_gain().unwrap_or_default() * 1000.) as i32
            });
            cards.reverse();
        }

        if cardqty == 0 {
            draw_message(stdout, "Nothing to review!");
            return;
        }

        for (index, card) in cards.into_iter().enumerate() {
            let info = cache.get_ref(&card).get_info(cache).unwrap_or_default();
            let status = format!(
                "{}/{}\t{}\t{}/{}/{}/{}/{}",
                index,
                cardqty,
                cache.get_ref(&card).category().print_full(),
                cache.dependencies(&card).len(),
                cache.dependents(&card).len(),
                (info.recall_rate * 100.).round(),
                (info.stability * 100.).round() / 100.,
                info.strength.round(),
            );

            match {
                match cache.get_ref(&card).get_review_type() {
                    ReviewType::Normal | ReviewType::Pending => {
                        review_card(stdout, &card, status.clone(), cache)
                    }

                    ReviewType::Unfinished => continue,
                }
            } {
                SomeStatus::Continue => {
                    continue;
                }
                SomeStatus::Break => return,
            }
        }
        if !toggle_refresh {
            return;
        }
    }
}

pub fn print_card_for_review(
    stdout: &mut Stdout,
    card: &SavedCard,
    show_backside: bool,
    status: &str,
) {
    execute!(stdout, Clear(ClearType::All)).unwrap();
    update_status_bar(stdout, status);
    print_card_review_front(stdout, card.card_as_ref(), true);
    if show_backside {
        print_card_review_back(stdout, card.card_as_ref(), true);
    }
}

pub fn review_card(
    stdout: &mut Stdout,
    card_id: &Id,
    status: String,
    cache: &mut CardCache,
) -> SomeStatus {
    let mut show_backside = false;
    let start_time = current_time();
    let mut duration = Duration::default();
    loop {
        let card = cache.get_ref(card_id);
        print_card_for_review(stdout, &card, show_backside, status.as_str());
        let keycode = get_keycode();
        if edit_card(stdout, &keycode, card.clone(), cache) {
            continue;
        }
        match keycode {
            KeyCode::Char('o') => view_all_cards(stdout, cache),
            KeyCode::Char('X') => {
                let _ = ascii_test(stdout, card.id(), cache, true);
            }
            KeyCode::Char('x') => {
                let _ = ascii_test(stdout, card.id(), cache, false);
            }
            KeyCode::Char('Y') => {
                draw_message(stdout, "Adding new dependency");
                let category = Some(card.category().to_owned());
                add_dependency(stdout, card.id(), category.as_ref(), cache);
            }
            KeyCode::Char('T') => {
                draw_message(stdout, "Adding new dependent");
                let category = Some(card.category().to_owned());
                add_dependent(stdout, card.id(), category.as_ref(), cache);
            }
            KeyCode::Char('q') => return SomeStatus::Break,
            KeyCode::Char('D') => {
                if affirmative(stdout, "Delete card?") {
                    cache.get_owned(card.id()).delete(cache);
                    draw_message(stdout, "Card deleted");
                    break;
                }
            }
            KeyCode::Char(' ') => {
                if !show_backside {
                    duration = current_time() - start_time;
                }
                show_backside = true;
            }
            KeyCode::Char('s') => break,
            KeyCode::Char('a') => {
                add_card(stdout, &mut card.category().to_owned(), cache);
            }
            KeyCode::Char(c) if show_backside => match c.to_string().parse() {
                Ok(grade) => {
                    cache.get_owned(card_id).new_review(grade, duration);
                    return SomeStatus::Continue;
                }
                _ => continue,
            },
            key if should_exit(&key) => return SomeStatus::Break,
            _ => continue,
        }
    }
    SomeStatus::Continue
}
