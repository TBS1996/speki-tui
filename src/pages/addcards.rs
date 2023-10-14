use std::io::Stdout;

use speki_backend::card::{Card, CardCache, SavedCard};
use speki_backend::categories::Category;

use speki_backend::Id;

use crate::backend::get_text_from_vim;

use super::draw_message;

pub fn add_card(
    initial_text: Option<(String, String)>,
    category: &mut Category,
    cache: &mut CardCache,
) -> Option<SavedCard> {
    let vimrc = r#"
function! AddTextAtBeginning()
    " Move the cursor to the beginning of the file
    normal gg
    normal o
    normal gg
    " Enter insert mode and add the text
    normal i<unfinished>
    normal j
    normal dd
    write
    quit
endfunction

function! SetSigns()
    set signcolumn=yes
    sign unplace *
    execute "sign define QSign text=Q: "
    execute "sign define ASign text=A: "
    execute "sign place 1 line=1 name=QSign"
    execute "sign place 2 line=2 name=ASign"
endfunction

highlight SignColumn guibg=none ctermbg=none

autocmd VimEnter * normal 50o
autocmd VimEnter * normal gg
autocmd VimEnter * call SetSigns()
autocmd BufWritePost,TextChanged,TextChangedI * call SetSigns()




nnoremap <C-f> :wq<CR>
inoremap <C-f> <Esc>:wq<CR>

nnoremap <C-q> :q!<CR>
inoremap <C-q> <Esc>:q!<CR>

inoremap <C-u> <Esc>:call AddTextAtBeginning()wq<CR>
nnoremap <C-u> :call AddTextAtBeginning()<CR>
"#;

    let initial_text = initial_text.map(|(mut front, back)| {
        front.push('\n');
        front.push_str(&back);
        front
    });

    let text = get_text_from_vim(initial_text, Some(vimrc)).ok()??;

    let (front, back) = match text.split_once('\n') {
        Some((front, back)) => (front.trim().to_string(), back.trim().to_string()),
        None => (text, String::default()),
    };

    let (front, is_finished) = match front.strip_prefix("<unfinished>") {
        Some(rest) => (rest.to_string(), false),
        None => (front, true),
    };

    let mut card = Card::new_simple(front, back);
    card.meta.finished = is_finished;

    Some(card.save_new_card(category, cache))
}

pub fn add_the_cards(_stdout: &mut Stdout, mut category: Category, cache: &mut CardCache) {
    while add_card(None, &mut category, cache).is_some() {}
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
    let new_dependency = add_card(None, category, cache)?;
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
    let new_dependent = add_card(None, &mut category, cache)?;
    let info = card.set_dependent(new_dependent.id(), cache);

    if let Some(info) = info {
        draw_message(stdout, &info);
    }
    cache.refresh();
    Some(new_dependent)
}
