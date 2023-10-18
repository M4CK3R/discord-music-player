use serenity::framework::standard::macros::group;

use crate::commands::{player::*, bot::*, queue::*};

#[group]
#[commands(join, leave, play, ping, shuffle, show, pause, resume, skip, set_loop, queue, add, remove, clear, move_song, stop, help, save, load, saved,remove_saved)]
struct General;