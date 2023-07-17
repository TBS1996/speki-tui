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

pub mod addcards;
pub mod reviewcards;
pub mod viewcards;

pub fn ascii_test(
    stdout: &mut Stdout,
    card_id: &Id,
    cache: &mut CardCache,
    show_dependencies: bool,
) -> Option<SavedCard> {
    let tree = to_ascii_tree(card_id, cache, show_dependencies, &mut BTreeSet::new());
    let mut output = String::new();

    let msg = if show_dependencies {
        "dependencies"
    } else {
        "dependents"
    };

    let _ = write_tree(&mut output, &tree);

    let lines: Vec<&str> = output.lines().collect();

    if lines.len() == 1 {
        draw_message(stdout, &format!("No {} found", msg));
        return None;
    }

    let item = pick_item(stdout, msg, &lines);

    if let Some(item) = item {
        let cards = if show_dependencies {
            cache.recursive_dependencies(card_id)
        } else {
            cache.recursive_dependents(card_id)
        };

        for card_id in cards {
            let card = cache.get_ref(&card_id);
            if item.contains(card.front_text()) {
                return Some(cache.get_owned(&card_id));
            }
        }
    }
    None
}

pub fn print_expected_stuff(stdout: &mut Stdout) {
    let mut cards: Vec<SavedCard> = SavedCard::load_all_cards()
        .into_iter()
        .filter(|card| card.stability().is_some())
        .collect();

    cards.sort_by_key(|card| (card.expected_gain().unwrap() * 1000.) as i32);

    let mut s = String::new();

    for card in cards {
        let (Some(gain), Some(stability), Some(recall)) = (card.expected_gain(), card.stability(), card.recall_rate()) else {continue};
        let gain = (gain * 100.).round() / 100.;
        let stability = (stability.as_secs_f32() / 864.).round() / 100.;
        let recall = (recall * 100.).round();
        let whatever = stability < card.time_since_last_review().unwrap().as_secs_f32() / 86400.;

        s.push_str(&format!(
            "gain: {}, stability: {}days, recall: {}%, hey: {}, card: {}\n",
            gain,
            stability,
            recall,
            whatever,
            card.front_text()
        ));
    }
    draw_message(stdout, s.as_str());
}

pub fn affirmative(stdout: &mut Stdout, question: &str) -> bool {
    match draw_menu(stdout, Some(question), vec!["no", "yes"], false).unwrap() {
        0 => false,
        1 => true,
        _ => unreachable!(),
    }
}

pub fn print_stats(stdout: &mut Stdout, cache: &mut CardCache) {
    let cards = SavedCard::load_all_cards();
    let all_cards = cards.len();
    let mut suspended = 0;
    let mut finished = 0;
    let mut pending = 0;
    let mut strength = 0;
    let mut reviews = 0;
    let mut resolved = 0;
    let mut daily_cards = 0.;

    for card in cards {
        pending += card.stability().is_none() as i32;
        reviews += card.reviews().len();
        finished += card.is_finished() as i32;
        resolved += card.is_resolved(cache) as i32;
        strength += (card.strength().unwrap_or_default().as_secs_f32() / 86400.).round() as i32;
        suspended += card.is_suspended() as i32;

        if let Some(stability) = card.stability() {
            if card.is_confidently_resolved(cache) {
                let mut days = 1. / duration_to_days(&stability);
                // if stability is like 0.1 it still feels dumb to say that it's on average 10 cards a day for one card lol
                if days > 1.0 {
                    days = 1.0;
                }
                daily_cards += days;
            }
        }
    }

    let output = format!("suspended: {suspended}\nfinished: {finished}\npending: {pending}\nreviews: {reviews}\nstrength: {strength}\nresolved: {resolved}\ndaily cards: {daily_cards}\ntotal cards: {all_cards}");
    draw_message(stdout, output.as_str());
    print_expected_stuff(stdout);

    let not_confident_cards: Vec<SavedCard> = SavedCard::load_all_cards()
        .into_iter()
        .filter(|card| {
            card.is_resolved(cache)
                && !card.is_confidently_resolved(cache)
                && card.is_finished()
                && !card.is_suspended()
        })
        .collect();
    let s = cards_as_string(&not_confident_cards);
    let s = format!("qty: {}\n{}", not_confident_cards.len(), s);
    draw_message(stdout, &s);
}

pub fn suspend_card(stdout: &mut Stdout, card: &Id, cache: &mut CardCache) {
    let mut card = cache.get_owned(card);
    draw_message(stdout, "hey how many days do you wanna suspend?");

    loop {
        if let Some((input, _)) = read_user_input(stdout) {
            if input.is_empty() {
                card.set_suspended(IsSuspended::True);
                draw_message(stdout, "Card suspended indefinitely");
                return;
            }

            if let Ok(num) = input.parse::<f32>() {
                let days = Duration::from_secs_f32(86400. * num);
                let until = days + current_time();
                card.set_suspended(IsSuspended::TrueUntil(until));
                draw_message(stdout, "Card suspended");
                return;
            }
        } else {
            draw_message(stdout, "Card not suspended");
            return;
        }
    }
}

pub fn health_check(stdout: &mut Stdout, cache: &mut CardCache) {
    cache.refresh();
    let all_cards = SavedCard::load_all_cards();
    move_upper_left(stdout);

    for mut card in all_cards {
        let _id = card.id().to_owned();
        let dependencies = card.dependency_ids().to_owned();
        let dependents = card.dependent_ids().to_owned();

        for d in dependencies {
            if !cache.exists(&d) {
                println!("dependency removed!");
                card.remove_dependency(&d, cache);
            }
        }

        for d in dependents {
            if !cache.exists(&d) {
                println!("dependent removed!");
                card.remove_dependent(&d, cache);
            }
        }

        for dependency in card.dependency_ids() {
            if !cache.exists(dependency) {
                //      card._remove_dependency(&id, cache);
            }

            //let mut dependency = cache._get_owned(dependency);
            //dependency.set_dependent(&id, cache);
        }

        for _dependent in card.dependent_ids() {
            //let mut dependent = cache._get_owned(dependent);
            //dependent.set_dependency(&id, cache);
        }
    }
    cache.refresh();
}

pub fn clear_window(stdout: &mut Stdout) {
    execute!(stdout, Clear(ClearType::All)).unwrap();
}

pub fn move_upper_left(stdout: &mut Stdout) {
    execute!(stdout, MoveTo(0, 0)).unwrap()
}

pub fn view_dependencies(stdout: &mut Stdout, card: &Id, cache: &mut CardCache) {
    let card = cache.get_owned(card);
    let mut msg = String::from("Dependents:\n");

    let dependents = cache.recursive_dependents(card.id());
    for dep in dependents {
        let dep = cache.get_ref(&dep);
        msg.push_str(&format!(
            "   {}\tfinished: {}\n",
            truncate_string(dep.front_text().to_owned(), 50),
            dep.is_finished(),
        ));
    }
    msg.push('\n');
    msg.push('\n');

    let dependencies = cache.recursive_dependencies(card.id());
    msg.push_str("Dependencies:\n");
    for dep in dependencies {
        let dep = cache.get_ref(&dep);
        msg.push_str(&format!(
            "   {}\tfinished: {}\n",
            truncate_string(dep.front_text().to_owned(), 50),
            dep.is_finished(),
        ));
    }

    draw_message(stdout, &msg);
}

pub fn print_the_cool_graphs(stdout: &mut Stdout, data: Vec<Vec<f64>>, message: &str) {
    let (_, height) = crossterm::terminal::size().unwrap();

    clear_window(stdout);
    move_upper_left(stdout);

    let output = rasciigraph::plot_many(
        data,
        rasciigraph::Config::default().with_height(height as u32 - 4),
    );

    let output = format!("{}\n_____________\n{}", message, output);

    write_string(stdout, &output);

    read().unwrap();
}

pub fn print_cool_graph(stdout: &mut Stdout, data: Vec<f64>, message: &str) {
    let (_, height) = crossterm::terminal::size().unwrap();

    clear_window(stdout);
    move_upper_left(stdout);

    let output = rasciigraph::plot(
        data,
        rasciigraph::Config::default().with_height(height as u32 - 4),
    );

    let output = format!("{}\n_____________\n{}", message, output);

    write_string(stdout, &output);

    read().unwrap();
}

pub fn print_cool_graphs(stdout: &mut Stdout, cache: &mut CardCache) {
    let mut all_cards = SavedCard::load_all_cards();
    all_cards.retain(|card| card.is_resolved(cache));

    let max = 300;
    let mut vec = vec![0; max];
    let mut max_stab = 0;

    for card in &all_cards {
        let Some(stability) = card.stability() else {continue};
        let stability = stability.as_secs() / (86400 / 4);
        if stability > max_stab {
            max_stab = stability;
        }
        if stability < max as u64 {
            vec[stability as usize] += 1;
        }
    }

    vec.truncate(max_stab as usize + 1);

    let newvec = vec.into_iter().map(|num| num as f64).collect();

    println!("{:?}", &newvec);
    print_cool_graph(stdout, newvec, "Stability distribution");

    let width = crossterm::terminal::size().unwrap().0 - 10;
    let mut rev_vec = vec![0; width as usize];

    for days in 0..width as u32 {
        print!("{} ", days);
        let mut count = 0;
        for card in &all_cards {
            let Some(mut time_passed)  = card.time_since_last_review() else {continue};
            time_passed += std::time::Duration::from_secs((86400 * days / 4).into());
            let Some(stability) = card.stability() else {continue};
            if Reviews::calculate_recall_rate(&time_passed, &stability) < 0.9 {
                count += 1;
            }
        }

        rev_vec[days as usize] = count;
    }

    let rev_vec: Vec<f64> = rev_vec.into_iter().map(|num| num as f64).collect();
    print_cool_graph(stdout, rev_vec, "Review distribution");

    let mut strengthvec = vec![0; 1000];
    let mut accum = vec![];

    let mut max_strength = 0;
    let mut tot_strength = 0.;
    for card in &all_cards {
        let Some(strength) = card.strength() else {continue};
        let strength = strength.as_secs_f32() / 86400.;
        /*
        println!(
            "stability: {}, passed: {}, strength {}, card: {} ",
            as_days(stability),
            as_days(&days_passed),
            strength,
            card.front_text()
        );
        move_far_left(stdout);
        */
        tot_strength += strength;
        let strength = strength as u32;
        if strength > max_strength {
            max_strength = strength;
        }
        strengthvec[strength as usize] += 1;
        accum.push(strength);
    }

    strengthvec.truncate(max_strength as usize + 50);

    let accum = strengthvec.into_iter().map(|num| num as f64).collect();

    //accum.sort();

    //let accum = accum.into_iter().map(|num| num as f64).collect();

    print_cool_graph(
        stdout,
        accum,
        &format!(
            "Strength distribution\ttot: {} days",
            (tot_strength / 1.) as u32
        ),
    );

    let mut recall_vec = vec![];
    for card in &all_cards {
        if let Some(recall) = card.recall_rate() {
            recall_vec.push((recall * 100.) as i32);
        }
    }

    recall_vec.sort_by(|a, b| b.cmp(a));
    recall_vec.retain(|num| *num % 2 == 0);
    let recall_vec = recall_vec.into_iter().map(|n| n as f64).collect();

    print_cool_graph(stdout, recall_vec, "Recall distribution");
}

pub fn print_card_review_front(stdout: &mut Stdout, card: &Card, sound: bool) {
    execute!(stdout, MoveTo(0, 1)).unwrap();
    println!("{}", card.front.text);
    if sound {
        //  card.front.audio.play_audio();
    }
}

pub fn print_card_review_back(stdout: &mut Stdout, card: &Card, sound: bool) {
    move_far_left(stdout);
    execute!(stdout, MoveDown(1)).unwrap();
    move_far_left(stdout);
    println!("------------------");
    execute!(stdout, MoveDown(1)).unwrap();
    move_far_left(stdout);
    println!("{}", card.back.text);
    move_far_left(stdout);

    if sound {
        //card.back.audio.play_audio();
    }
}

pub fn view_card_info(stdout: &mut Stdout, card: Arc<SavedCard>) {
    if card.reviews().is_empty() {
        return;
    }
    let the_current_time = current_time();

    let mut recall_rates: Vec<f64> = vec![];
    let mut failcall_rates: Vec<f64> = vec![];
    let mut wincall_rates: Vec<f64> = vec![];

    let win_and_fail_cards = |time: Duration, card: &SavedCard| -> (SavedCard, SavedCard) {
        let win_card = {
            let mut thecard = (*card).clone();
            thecard.fake_new_review(speki_backend::card::Grade::Some, Duration::default(), time);
            thecard
        };

        let fail_card = {
            let mut thecard = (*card).clone();
            thecard.fake_new_review(speki_backend::card::Grade::Late, Duration::default(), time);
            thecard
        };

        (win_card, fail_card)
    };

    let win_card = {
        let mut thecard = (*card).clone();
        thecard.fake_new_review(
            speki_backend::card::Grade::Some,
            Duration::default(),
            current_time(),
        );
        thecard
    };

    let fail_card = {
        let mut thecard = (*card).clone();
        thecard.fake_new_review(
            speki_backend::card::Grade::Late,
            Duration::default(),
            current_time(),
        );
        thecard
    };

    for i in 0..200 {
        let time = the_current_time + Duration::from_secs(1 + 86400 * i);
        dbg!();
        let x = card.ml_recall_rate_at_time(time).unwrap() as f64;
        recall_rates.push(x);

        let x = win_card.ml_recall_rate_at_time(time).unwrap() as f64;
        dbg!();
        wincall_rates.push(x);
        let x = fail_card.ml_recall_rate_at_time(time).unwrap() as f64;
        dbg!();
        failcall_rates.push(x);
    }

    failcall_rates[100] = 1.0;

    //print_cool_graph(stdout, recall_rates, "recall rate hey");

    draw_message(stdout, "this might take a  while...");
    let mut current_strength: Vec<f64> = vec![];
    let mut win_strength: Vec<f64> = vec![];
    let mut fail_strength: Vec<f64> = vec![];
    let mut gainstuff = vec![];
    let mut strength_over_time = vec![];

    for i in 0..200 {
        if i % 2 == 0 {
            continue;
        }
        let time = current_time() + Duration::from_secs(86400 * i);

        let (win_card, fail_card) = win_and_fail_cards(time, &(*card).clone());
        let x = SavedCard::pure_ml_expected_gain((*card).clone(), win_card, fail_card, time);
        gainstuff.push(x as f64);

        let x = card.ml_expected_gain_debug(time).unwrap();
        current_strength.push(duration_to_days(&x.0).into());
        win_strength.push(duration_to_days(&x.1).into());
        fail_strength.push(duration_to_days(&x.2).into());
        strength_over_time.push(duration_to_days(&card.strength_at_time(time).unwrap()) as f64)
    }

    dbg!(&current_strength, &win_strength, &fail_strength);

    let graphs = vec![current_strength, win_strength, fail_strength];
    // let graphs = vec![current_strength, fail_strength];

    let recall_rates = vec![recall_rates, wincall_rates, failcall_rates];
    print_the_cool_graphs(stdout, recall_rates, "some recall rates");

    print_the_cool_graphs(stdout, graphs, "strength over time");

    print_cool_graph(stdout, gainstuff, "gain over time ig");

    print_cool_graph(stdout, strength_over_time, "strength over time");
}

/// Bool represents if any action was taken.
pub fn edit_card(
    stdout: &mut Stdout,
    key: &KeyCode,
    card: Arc<SavedCard>,
    cache: &mut CardCache,
) -> bool {
    let mut excluded_cards = HashSet::new();
    excluded_cards.insert(card.id().to_owned());
    match key {
        KeyCode::Char('`') => {
            let info = format!("{:?}", card.get_info(cache));
            draw_message(stdout, info.as_str());
            view_card_info(stdout, card);
        }
        KeyCode::Char('p') => {
            let ch = _get_char();
            if let Ok(priority) = ch.try_into() {
                cache.get_owned(card.id()).set_priority(priority);
            }
        }

        KeyCode::Char('P') => {
            draw_message(stdout, "choose priority, from 0 to 100");
            if let Some(input) = read_user_input(stdout) {
                if let Ok(num) = input.0.trim().parse::<u32>() {
                    let priority: Priority = num.into();
                    cache.get_owned(card.id()).set_priority(priority);
                }
            }
        }

        KeyCode::Char('f') => {
            let mut thecard = cache.get_owned(card.id());
            thecard.set_finished(true);
        }

        KeyCode::Char('S') => suspend_card(stdout, card.id(), cache),

        KeyCode::Char('g') => {
            let tags = card.category().get_tags().into_iter().collect();
            let tag = match pick_item(stdout, "Choose tag", &tags) {
                Some(tag) => tag,
                None => return true,
            };
            let mut thecard = cache.get_owned(card.id());
            thecard.insert_tag(tag.to_owned());
        }

        KeyCode::Char('y') => {
            if let Some(chosen_card) = search_for_item(stdout, "Add dependency", excluded_cards) {
                cache
                    .get_owned(card.id())
                    .set_dependency(chosen_card.id(), cache);
                cache.refresh();
            }
        }
        KeyCode::Char('t') => {
            if let Some(chosen_card) = search_for_item(stdout, "Add dependent", excluded_cards) {
                let info = cache
                    .get_owned(card.id())
                    .set_dependent(chosen_card.id(), cache);
                if let Some(info) = info {
                    draw_message(stdout, &info);
                }
            }
        }
        KeyCode::Char('v') => {
            view_dependencies(stdout, card.id(), cache);
        }
        KeyCode::Char('m') => {
            let folder = match choose_folder(stdout, "Move card to...") {
                Some(folder) => folder,
                None => return true,
            };

            let moved_card = cache.get_owned(card.id()).move_card(&folder, cache);
            cache.insert(moved_card);
        }
        KeyCode::Char('e') => {
            card.edit_with_vim();
        }
        _ => return false,
    };
    true
}

/// Fixes the problem where printing a newline doesn't make the cursor go to the left
pub fn write_string(stdout: &mut Stdout, message: &str) {
    for char in message.chars() {
        print!("{char}");
        if char == '\n' {
            move_far_left(stdout);
        }
    }
}

pub fn draw_menu(
    stdout: &mut Stdout,
    message: Option<&str>,
    items: Vec<&str>,
    optional: bool,
) -> Option<usize> {
    let mut selected = 0;

    loop {
        execute!(stdout, Clear(ClearType::All)).unwrap();
        move_upper_left(stdout);
        if let Some(message) = message {
            println!("{message}");
        }

        for (index, item) in items.iter().enumerate() {
            execute!(stdout, MoveTo(0, index as u16 + 1)).unwrap();

            if index == selected {
                execute!(stdout, SetForegroundColor(crossterm::style::Color::Blue)).unwrap();
                println!("> {}", item);
                execute!(stdout, ResetColor).unwrap();
            } else {
                println!("  {}", item);
            }
        }

        // Await input from user
        if let Event::Key(event) = read().unwrap() {
            match event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < items.len() - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Char('G') => selected = items.len() - 1,
                KeyCode::Enter | KeyCode::Char(' ') => {
                    execute!(stdout, Clear(ClearType::All)).unwrap();
                    execute!(stdout, MoveTo(0, items.len() as u16 + 1)).unwrap();
                    return Some(selected);
                }
                KeyCode::Char('q') | KeyCode::Esc if optional => return None,
                _ => {}
            }
        }
    }
}

pub fn move_far_left(stdout: &mut Stdout) {
    let (_, y) = cursor::position().unwrap();
    execute!(stdout, MoveTo(0, y)).unwrap();
}

pub fn update_status_bar(stdout: &mut Stdout, msg: &str) {
    let pre_pos = cursor::position().unwrap();
    execute!(stdout, MoveTo(0, 0)).unwrap();
    writeln!(stdout, "{}", msg).unwrap();
    stdout.flush().unwrap();
    execute!(stdout, cursor::MoveTo(pre_pos.0, pre_pos.1)).unwrap();
}

pub fn draw_key_event_message(stdout: &mut Stdout, message: &str) -> KeyEvent {
    execute!(stdout, MoveTo(0, 0)).unwrap();

    execute!(stdout, Clear(ClearType::All)).unwrap();
    write_string(stdout, message);
    execute!(stdout, ResetColor).unwrap();

    let pressed_char = get_key_event();

    execute!(stdout, Clear(ClearType::All)).unwrap();

    pressed_char
}

pub fn draw_message(stdout: &mut Stdout, message: &str) -> KeyCode {
    execute!(stdout, MoveTo(0, 0)).unwrap();

    execute!(stdout, Clear(ClearType::All)).unwrap();
    write_string(stdout, message);
    execute!(stdout, ResetColor).unwrap();

    let pressed_char = get_keycode();

    execute!(stdout, Clear(ClearType::All)).unwrap();

    pressed_char
}

pub fn choose_folder(stdout: &mut Stdout, message: &str) -> Option<Category> {
    pick_item_with_formatter(
        stdout,
        message,
        &Category::load_all().unwrap(),
        Category::print_it_with_depth,
    )
    .cloned()
}

pub fn pick_item<'a, T: Display>(
    stdout: &mut Stdout,
    message: &str,
    items: &'a Vec<T>,
) -> Option<&'a T> {
    let formatter = |item: &T| format!("{}", item);
    pick_item_with_formatter(stdout, message, items, formatter)
}

pub fn pick_item_with_formatter<'a, T, F>(
    stdout: &mut Stdout,
    message: &str,
    items: &'a Vec<T>,
    formatter: F,
) -> Option<&'a T>
where
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        draw_message(stdout, "list is empty");
        return None;
    }
    let mut selected = 0;

    loop {
        execute!(stdout, Clear(ClearType::All)).unwrap();
        execute!(stdout, MoveTo(0, 0)).unwrap();
        print!("{}", message);

        for (index, item) in items.iter().enumerate() {
            execute!(stdout, MoveTo(0, (index + 1) as u16)).unwrap();

            if index == selected {
                execute!(stdout, SetForegroundColor(crossterm::style::Color::Blue)).unwrap();
                println!("> {}", formatter(item));
                execute!(stdout, ResetColor).unwrap();
            } else {
                println!("  {}", formatter(item));
            }
        }

        // Await input from user
        if let Event::Key(event) = read().unwrap() {
            match event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Char('G') => selected = items.len() - 1,
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < items.len() - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => return Some(&items[selected]),
                key if should_exit(&key) => return None,
                _ => {}
            }
        }
    }
}

pub fn search_for_item(
    stdout: &mut Stdout,
    message: &str,
    excluded_cards: HashSet<Id>,
) -> Option<SavedCard> {
    let mut input = String::new();

    let cards = SavedCard::load_all_cards();
    let mut index = 0;

    let mut print_stuff = |search_term: &str, cards: Vec<&SavedCard>, index: &mut usize| {
        clear_window(stdout);
        //move_upper_left(stdout);
        execute!(stdout, MoveTo(0, 0)).unwrap();
        println!("{}", message);
        println!("\t\t| {} |", search_term);
        let screen_height = crossterm::terminal::size().unwrap().1 - 10;
        *index = std::cmp::min(
            std::cmp::min(*index, screen_height.into()),
            cards.len().saturating_sub(1),
        );
        for (idx, card) in cards.iter().enumerate() {
            move_far_left(stdout);

            if idx == *index {
                execute!(stdout, SetForegroundColor(crossterm::style::Color::Blue)).unwrap();
                println!("> {}", card.front_text());
                execute!(stdout, ResetColor).unwrap();
            } else {
                println!("  {}", card.front_text());
            }

            if idx == screen_height.into() {
                break;
            }
        }
    };

    loop {
        if let Event::Key(event) = read().unwrap() {
            match event.code {
                KeyCode::Char(c) => {
                    input.push(c);
                    let the_cards = SavedCard::search_in_cards(&input, &cards, &excluded_cards);
                    print_stuff(&input, the_cards, &mut index);
                }
                KeyCode::Backspace if !input.is_empty() => {
                    input.pop();
                    let the_cards = SavedCard::search_in_cards(&input, &cards, &excluded_cards);
                    print_stuff(&input, the_cards, &mut index);
                }
                KeyCode::Enter => {
                    let the_cards = SavedCard::search_in_cards(&input, &cards, &excluded_cards);
                    if the_cards.is_empty() {
                        return None;
                    }
                    return Some(the_cards[index].to_owned());
                }
                KeyCode::Down => {
                    index += 1;

                    let the_cards = SavedCard::search_in_cards(&input, &cards, &excluded_cards);
                    print_stuff(&input, the_cards, &mut index);
                }
                KeyCode::Up => {
                    let the_cards = SavedCard::search_in_cards(&input, &cards, &excluded_cards);
                    index = index.saturating_sub(1);
                    print_stuff(&input, the_cards, &mut index);
                }
                KeyCode::Esc => return None,
                _ => {}
            }
        }
    }
}

pub fn read_user_input(stdout: &mut Stdout) -> Option<(String, KeyCode)> {
    let mut input = String::new();
    let mut key_code;

    loop {
        if let Event::Key(event) = read().unwrap() {
            key_code = event.code;
            match event.code {
                KeyCode::Char('`') => break,
                KeyCode::Char(c) => {
                    input.push(c);
                    // You can decide whether to echo the input to the screen or not
                    print!("{}", c);
                    stdout.flush().unwrap(); // Make sure the char is displayed
                }
                KeyCode::Backspace if !input.is_empty() => {
                    input.pop();
                    let (x, y) = cursor::position().unwrap();

                    if x == 0 && y != 0 {
                        let (width, _) = terminal::size().unwrap();
                        execute!(stdout, MoveTo(width, y - 1), Print(" "),).unwrap();
                    } else {
                        execute!(stdout, MoveLeft(1), Print(" "), MoveLeft(1),).unwrap();
                    }
                    stdout.flush().unwrap();
                }
                KeyCode::Enter => break,
                KeyCode::Tab => break,
                KeyCode::Esc => return None,
                KeyCode::F(1) => break,
                _ => {}
            }
        }
    }
    Some((input, key_code))
}

use std::io::{Stdout, Write};

use crate::backend::{
    _get_char, cards_as_string, get_key_event, get_keycode, should_exit, to_ascii_tree,
    CardsFromCategory,
};

pub enum SomeStatus {
    Continue,
    Break,
}
