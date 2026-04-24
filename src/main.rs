use std::io::{self, Read};

mod git;
mod input;
mod render;
mod transcript;
mod util;
mod version;

fn main() {
    let mut buf = String::new();
    let _ = io::stdin().read_to_string(&mut buf);
    let parsed: input::Input = serde_json::from_str(&buf).unwrap_or_default();
    let line = render::render(&parsed);
    print!("{}", line);
}
