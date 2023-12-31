//! this will be about actually using the program like reviewing and all that

use crate::pages::health_check;
use crate::pages::pick_item;
use crate::pages::print_cool_graphs;
use crate::pages::print_stats;
use crate::pages::viewcards::view_all_cards;
use crate::pages::viewcards::view_cards;
use std::io::stdout;

use crate::pages::reviewcards::review_cards;

use backend::{get_following_unfinished_cards, import_stuff};
use pages::addcards::add_the_cards;
use pages::{choose_folder, draw_menu, draw_message};
use speki_backend::card::{CardCache, SavedCard};
use speki_backend::categories::Category;
use speki_backend::common::view_cards_in_explorer;
use speki_backend::common::{open_file_with_vim, randvec};
use speki_backend::config::Config;
use speki_backend::git::git_save;

use speki_backend::paths::get_share_path;

use crossterm::cursor::Show;

use crossterm::{
    cursor::Hide,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

mod backend;
mod pages;

trait Page {}

struct App {
    pages: Vec<Box<dyn Page>>,
}

#[tokio::main]
async fn main() {
    let mut cache = CardCache::new();
    import_stuff(&mut cache);

    enable_raw_mode().unwrap();
    let mut stdout = stdout();
    execute!(stdout, Hide).unwrap();

    /*
    two_review_stuff();
    three_review_stuff();
    fourplus_review_stuff();
    five_review_stuff();
    six_review_stuff();
    */

    let menu_items = vec![
        "Add new cards",
        "Review cards",
        "View cards",
        "Settings",
        "Debug",
        "by tag",
        "notes",
        "pretty graph",
        "lonely cards",
        "health check",
        "stats",
        "filters",
    ];

    while let Some(choice) = draw_menu(&mut stdout, None, menu_items.clone(), true) {
        match choice {
            0 => {
                let Some(category) = choose_folder(&mut stdout, "Folder to add card to") else {
                    continue;
                };
                add_the_cards(&mut stdout, category, &mut cache);
                let has_remote = Config::load().unwrap().git_remote.is_some();
                let _ = std::thread::spawn(move || git_save(has_remote));
            }
            1 => {
                let Some(revtype) = draw_menu(
                    &mut stdout,
                    None,
                    vec!["Normal", "Pending", "Unfinished", "Random review"],
                    true,
                ) else {
                    continue;
                };

                let Some(category) = choose_folder(&mut stdout, "Choose review type") else {
                    continue;
                };

                match revtype {
                    0 => {
                        review_cards(
                            &mut stdout,
                            category.clone(),
                            Box::new(Category::get_review_cards),
                            &mut cache,
                            true,
                            false,
                        );
                        draw_message(&mut stdout, "now reviewing pending cards");
                        review_cards(
                            &mut stdout,
                            category.clone(),
                            Box::new(Category::get_pending_cards),
                            &mut cache,
                            true,
                            false,
                        );
                    }
                    1 => {
                        review_cards(
                            &mut stdout,
                            category.clone(),
                            Box::new(Category::get_pending_cards),
                            &mut cache,
                            true,
                            false,
                        );
                    }
                    2 => {
                        let mut cards = get_following_unfinished_cards(&category, &mut cache);
                        cards.sort_by_key(|card| {
                            cache.get_ref(card).get_unfinished_dependent_qty(&mut cache)
                        });
                        cards.reverse();
                        view_cards(&mut stdout, cards, &mut cache);
                    }
                    3 => {
                        review_cards(
                            &mut stdout,
                            category.clone(),
                            Box::new(Category::get_random_review_cards),
                            &mut cache,
                            false,
                            true,
                        );
                    }
                    _ => continue,
                }

                let has_remote = Config::load().unwrap().git_remote.is_some();
                let _ = std::thread::spawn(move || git_save(has_remote));
            }
            2 => view_cards_in_explorer(),
            3 => {
                let _ = Config::edit_with_vim();
            }
            4 => {
                view_all_cards(&mut stdout, &mut cache);
            }
            5 => {
                let tags: Vec<String> = Category::get_all_tags().into_iter().collect();
                let tag = pick_item(&mut stdout, "Tag to filter by", &tags);
                if let Some(tag) = tag {
                    let cards = SavedCard::load_all_cards()
                        .into_iter()
                        .filter_map(|card| card.contains_tag(tag).then(|| card.id().to_owned()))
                        .collect();
                    view_cards(&mut stdout, cards, &mut cache);
                }
            }
            6 => {
                open_file_with_vim(get_share_path().join("notes").as_path()).unwrap();
            }
            7 => {
                print_cool_graphs(&mut stdout, &mut cache);
            }
            8 => {
                let mut cards = SavedCard::load_all_cards()
                    .into_iter()
                    .collect::<Vec<SavedCard>>();
                cards.retain(|card| {
                    card.dependency_ids().is_empty()
                        && card.dependent_ids().is_empty()
                        && card.is_finished()
                });
                let cards = randvec(cards);
                let cards = cards.into_iter().map(|card| card.id().to_owned()).collect();
                view_cards(&mut stdout, cards, &mut cache);
            }
            9 => {
                health_check(&mut stdout, &mut cache);
            }
            10 => print_stats(&mut stdout, &mut cache),
            _ => {}
        };
    }
    execute!(stdout, Clear(ClearType::All)).unwrap();
    execute!(stdout, Show).unwrap();
    disable_raw_mode().unwrap();
}
