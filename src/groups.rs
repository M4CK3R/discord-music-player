use serenity::framework::standard::macros::group;

use crate::commands::{bot::*, player::*, queue::*};

#[group]
#[commands(join, leave, ping/*, help*/)]
#[description = "Bot commands"]
#[summary = "Manage the bot"]
#[only_in(guilds)]
struct General;

#[group]
#[commands(pause, resume, set_loop, skip/*, stop, play*/)]
#[description = "Player commands"]
#[summary = "Start, stop, and control the music player"]
#[only_in(guilds)]
struct Player;

#[group]
#[commands(shuffle, show, queue, add, clear, remove, move_song)]
#[description = "Queue commands"]
#[summary = "Add, remove, and view the queue"]
#[only_in(guilds)]
struct Queue;

#[group]
#[commands(save, load, saved, remove_saved)]
#[description = "Cache commands"]
#[summary = "Save and load the cache"]
#[only_in(guilds)]
struct Cache;
