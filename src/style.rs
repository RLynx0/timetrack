use owo_colors::{OwoColorize, Stream};

pub fn green(text: &str) -> String {
    text.if_supports_color(Stream::Stdout, |value| value.green())
        .to_string()
}

pub fn red(text: &str) -> String {
    text.if_supports_color(Stream::Stdout, |value| value.red())
        .to_string()
}
