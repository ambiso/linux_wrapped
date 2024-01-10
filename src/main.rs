use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Split},
    iter::Peekable,
};

use rand::seq::SliceRandom;

trait CommandHistory: Iterator<Item = Vec<u8>> {}

impl<T: Iterator<Item = Vec<u8>>> CommandHistory for T {}

struct ZshHistory {
    lines: Peekable<Split<BufReader<File>>>,
}

impl ZshHistory {
    fn new() -> Option<Self> {
        let mut path = home::home_dir()?;
        path.push(".zsh_history");
        let f = File::open(path).ok()?;
        let br = BufReader::new(f);
        Some(Self {
            lines: br.split(b'\n').peekable(),
        })
    }
}

impl Iterator for ZshHistory {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.lines.next();
            item.as_ref()?;
            if let Some(mut result) = item.transpose().ok().flatten().and_then(|x| {
                if let Some(b':') = x.first() {
                    return None;
                }
                let mut it = x.split(|x| *x == b';');
                it.next();
                it.next().map(|x| x.to_owned())
            }) {
                while let Some(next) = self.lines.peek().as_ref() {
                    if next.as_ref().unwrap().starts_with(b":") {
                        break;
                    }
                    result.extend(&self.lines.next().unwrap().unwrap());
                }

                return Some(result);
            }
        }
    }
}

#[derive(Default)]
struct State {
    man_pages: HashMap<String, u32>,
    git_subcommands: HashMap<String, u32>,
    commands: HashMap<String, u32>,
}

fn process_command_history(state: &mut State, command_history: &mut dyn CommandHistory) {
    for entry in command_history {
        let mut it = entry.split(|x| *x == b' ');
        let mut cmd = it.next();
        while let Some(icmd) = cmd {
            // incomplete parsing of env-vars (in theory this could be an escaped equals sign, or part of a string)
            if icmd.contains(&b'=') {
                cmd = it.next();
            } else {
                break;
            }
        }
        let arg1 = it.next();
        let arg2 = it.next();
        (|| {
            if let (Some(b"man"), Some(arg1), arg2) = (cmd, arg1, arg2) {
                let mut page = arg1;
                if arg1.iter().all(|x| x.is_ascii_digit()) {
                    if let Some(arg2) = arg2 {
                        page = arg2;
                    } else {
                        return Ok(());
                    }
                }
                *state
                    .man_pages
                    .entry(String::from_utf8(page.to_owned())?)
                    .or_default() += 1;
            }
            Result::<(), Box<dyn std::error::Error>>::Ok(())
        })()
        .ok();
        (|| {
            match (cmd, arg1) {
                (Some(b"g" | b"git"), Some(subcommand)) => {
                    *state
                        .git_subcommands
                        .entry(String::from_utf8(subcommand.to_owned())?)
                        .or_default() += 1;
                }
                (Some(b"gc" | b"gca"), _) => {
                    *state
                        .git_subcommands
                        .entry(String::from_utf8(b"commit".to_vec())?)
                        .or_default() += 1;
                }
                (Some(b"ga" | b"gau"), _) => {
                    *state
                        .git_subcommands
                        .entry(String::from_utf8(b"add".to_vec())?)
                        .or_default() += 1;
                }
                _ => {}
            }
            Result::<(), Box<dyn std::error::Error>>::Ok(())
        })()
        .ok();
        (|| {
            if let Some(cmd) = cmd {
                *state
                    .commands
                    .entry(String::from_utf8(cmd.to_owned())?)
                    .or_default() += 1;
            };

            Result::<(), Box<dyn std::error::Error>>::Ok(())
        })()
        .ok();
    }
}

fn main() {
    let mut rng = rand::thread_rng();
    let mut state = State::default();
    if let Some(mut h) = ZshHistory::new() {
        process_command_history(&mut state, &mut h);
    }

    let mut most_used_man_pages: Vec<_> = state.man_pages.iter().map(|x| (x.1, x.0)).collect();
    let lookups: u64 = state.man_pages.iter().map(|x| *x.1 as u64).sum();
    if lookups > 0 {
        most_used_man_pages.sort_unstable();
        println!("You looked up manual pages a total of {lookups} times! #RTFM");
        println!(
            "{}",
            [
                "You just couldn't get enough of reading these manuals:",
                "In desperate times you turned to these man pages:"
            ]
            .choose(&mut rng)
            .unwrap()
        );
        for (count, man_page) in most_used_man_pages.iter().rev().take(15) {
            println!("{count} {man_page}");
        }
        println!();
    }

    let mut most_used_subcommands: Vec<_> =
        state.git_subcommands.iter().map(|x| (x.1, x.0)).collect();
    most_used_subcommands.sort_unstable();
    println!("Your favorite git subcommands are:");
    for (count, cmd) in most_used_subcommands.iter().rev().take(5) {
        println!("{count} {cmd}");
    }
    println!();

    let mut most_used_commands: Vec<_> = state.commands.iter().map(|x| (x.1, x.0)).collect();
    most_used_commands.sort_unstable();
    println!("Your top commands are:");
    for (count, cmd) in most_used_commands.iter().rev().take(15) {
        println!("{count} {cmd}");
    }
    println!("... maybe consider sponsoring them?");
}
