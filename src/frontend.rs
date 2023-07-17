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
