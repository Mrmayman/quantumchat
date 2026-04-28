//! Whatsapp-compliant rich text rendering.

use std::collections::HashSet;

use iced::widget;

use crate::{FONT_DEFAULT, FONT_EMOJI, FONT_MONO};

// TODO: three backticks

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct TextStyle {
    bold: bool,
    italic: bool,
    strikethrough: bool,
    monospace: bool,
    /// See [`StyleType::Emoji`] for more info
    emoji: bool,
}

impl std::fmt::Debug for TextStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Bold;
        #[derive(Debug)]
        struct Italic;
        #[derive(Debug)]
        struct Strikethrough;
        #[derive(Debug)]
        struct Monospace;
        #[derive(Debug)]
        struct Emoji;

        let mut l = f.debug_list();
        if self.bold {
            l.entry(&Bold);
        }
        if self.italic {
            l.entry(&Italic);
        }
        if self.strikethrough {
            l.entry(&Strikethrough);
        }
        if self.monospace {
            l.entry(&Monospace);
        }
        if self.emoji {
            l.entry(&Emoji);
        }
        l.finish()
    }
}

impl TextStyle {
    fn apply(&mut self, style_type: StyleType, apply: bool) {
        match style_type {
            StyleType::Bold => self.bold = apply,
            StyleType::Italic => self.italic = apply,
            StyleType::Strikethrough => self.strikethrough = apply,
            StyleType::Monospace => self.monospace = apply,
            StyleType::Emoji => self.emoji = apply,
        }
    }

    fn get(&self, style_type: StyleType) -> bool {
        match style_type {
            StyleType::Bold => self.bold,
            StyleType::Italic => self.italic,
            StyleType::Strikethrough => self.strikethrough,
            StyleType::Monospace => self.monospace,
            StyleType::Emoji => self.emoji,
        }
    }

    fn get_font(self) -> iced::Font {
        if self.emoji {
            FONT_EMOJI
        } else if self.monospace {
            FONT_MONO
        } else {
            FONT_DEFAULT
        }
    }
}

fn is_emoji(c: char) -> bool {
    matches!(
        c as u32,
        0x1F300..=0x1FAFF | // emoji blocks
        0x2600..=0x26FF   | // misc symbols
        0x2700..=0x27BF     // dingbats
    )
}

fn is_sticky(c: char) -> bool {
    c.is_whitespace()
        || StyleType::from_char(c).is_some()
        || [
            '-', '=', '+', '!', '@', '#', '$', '%', '^', '&', '(', ')', '{', '}', '[', ']', ';',
            '\'', ':', '\"', '\\', '|', ',', '.', '/', '<', '>', '?',
        ]
        .contains(&c)
}

#[derive(Debug, PartialEq)]
enum Command {
    AddText(String),
    Start(StyleType),
    End(StyleType),
    Newline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum StyleType {
    Bold,
    Italic,
    Strikethrough,
    Monospace,
    /// # Why have emoji as style?
    ///
    /// In iced, you have to split emojis into separate text `Span`s
    /// to apply the emoji-specific font. Otherwise,
    /// it breaks in platform-specific ways.
    ///
    /// Since we anyway have to split it out,
    /// might as well reuse existing logic for this.
    Emoji,
}

impl StyleType {
    fn from_char(c: char) -> Option<Self> {
        Some(match c {
            '*' => Self::Bold,
            '_' => Self::Italic,
            '~' => Self::Strikethrough,
            '`' => Self::Monospace,
            _ => return None,
        })
    }

    fn to_char(self) -> Option<char> {
        Some(match self {
            Self::Bold => '*',
            Self::Italic => '_',
            Self::Strikethrough => '~',
            Self::Monospace => '`',
            Self::Emoji => return None,
        })
    }
}

pub fn rich_text(input: &str) -> Vec<widget::text::Span<'static>> {
    let mut spans = Vec::new();

    let spans_raw = generate_spans(input);
    for (text, style) in spans_raw {
        let mut font = style.get_font();
        if !style.emoji {
            if style.bold {
                font.weight = iced::font::Weight::Bold;
            }
            if style.italic {
                font.style = iced::font::Style::Italic;
            }
        }

        let span = widget::span(text)
            .font(font)
            .strikethrough(style.strikethrough && !style.emoji);
        spans.push(span);
    }
    spans
}

fn generate_spans(input: &str) -> Vec<(String, TextStyle)> {
    let mut spans = Vec::new();

    // println!("========");
    let mut commands = generate_span_commands(input);
    clean_up_span_commands(&mut commands);
    merge_commands(&mut commands);

    let mut style = TextStyle::default();
    for command in commands {
        match command {
            Command::AddText(s) => spans.push((s, style)),
            Command::Start(style_type) => style.apply(style_type, true),
            Command::End(style_type) => style.apply(style_type, false),
            Command::Newline => spans.push(("\n".to_string(), style)),
        }
    }

    spans
}

/// Merges adjacent [`Command::AddText`] commands into a single one.
fn merge_commands(commands: &mut Vec<Command>) {
    let mut out = Vec::new();
    let mut buf = String::new();

    for cmd in commands.drain(..) {
        match cmd {
            Command::AddText(n) => buf.push_str(&n),
            Command::Newline => buf.push('\n'),
            c @ Command::Start(_) | c @ Command::End(_) => {
                if !buf.is_empty() {
                    out.push(Command::AddText(std::mem::take(&mut buf)));
                }
                out.push(c);
            }
        }
    }
    if !buf.is_empty() {
        out.push(Command::AddText(std::mem::take(&mut buf)));
    }

    *commands = out;
}

/// Removes mismatched start/end tags from the formatted text,
/// and adds them back as regular characters (`*`, `~`, etc.) if needed.
///
/// I know this isn't the most elegant solution,
/// but it was the easiest way to fully match WhatsApp's logic.
fn clean_up_span_commands(commands: &mut Vec<Command>) {
    let mut stack = Vec::new();
    let mut to_remove = Vec::new();

    for (i, cmd) in commands.iter().enumerate() {
        match cmd {
            Command::Start(s) => stack.push((i, *s)),
            Command::End(s) => {
                let mut found = false;
                while let Some((idx, item)) = stack.pop() {
                    if item == *s {
                        found = true;
                        break;
                    }
                    to_remove.push(idx);
                }
                if !found {
                    to_remove.push(i);
                }
            }
            Command::Newline => to_remove.extend(stack.iter().map(|(i, _)| *i)),
            Command::AddText(_) => {}
        }
    }
    to_remove.extend(stack.into_iter().map(|(i, _)| i));
    remove_duplicates(&mut to_remove);

    for idx in to_remove.into_iter().rev() {
        let cmd = commands.remove(idx);
        if let Command::Start(s) | Command::End(s) = cmd
            && let Some(c) = s.to_char()
        {
            commands.insert(idx, Command::AddText(c.to_string()));
        }
    }
}

fn remove_duplicates<T: Eq + Clone + std::hash::Hash>(vec: &mut Vec<T>) {
    let mut unique_set = HashSet::new();
    vec.retain(|n| unique_set.insert(n.clone()));
}

fn generate_span_commands(input: &str) -> Vec<Command> {
    let mut text = String::new();
    let mut commands = Vec::new();

    // WhatsApp trims it too
    let chars: Vec<char> = input.trim().chars().collect();

    let mut just_added = None;
    let mut temp_style = TextStyle::default();
    let ended = parse_beginning(&mut commands, &chars, &mut just_added, &mut temp_style);
    if ended {
        return commands;
    }

    let mut iter = chars.windows(3);
    let mut in_monospace = false;

    while let Some([c1, c2, c3]) = iter.next() {
        // println!("    \"{c1}{c2}{c3}\"");
        if let Some(s) = StyleType::from_char(*c2) {
            // println!("{s:?}");
            let was_just_added = just_added.is_some_and(|j| j == s);
            // println!(
            //     "  just added: {was_just_added}, active: {}",
            //     temp_style.get(s)
            // );
            if is_sticky(*c1) && !c3.is_whitespace() && !was_just_added && !temp_style.get(s) {
                // println!("  1");
                if s == StyleType::Monospace {
                    in_monospace = true;
                } else if in_monospace {
                    continue;
                }
                flush_buf(&mut text, &mut commands);
                commands.push(Command::Start(s));
                temp_style.apply(s, true);
                just_added = Some(s);
                continue;
            } else if !c1.is_whitespace() && is_sticky(*c3) && !was_just_added {
                // println!("  2");
                just_added = None;
                flush_buf(&mut text, &mut commands);
                if s == StyleType::Monospace {
                    in_monospace = false;
                } else if in_monospace {
                    continue;
                }
                commands.push(Command::End(s));
                temp_style.apply(s, false);
                continue;
            }
            // println!("  3");
        }
        just_added = None;

        if *c2 == '\n' || *c2 == '\r' {
            flush_buf(&mut text, &mut commands);
            commands.push(Command::Newline);
            just_added = None;
            temp_style = TextStyle::default();
        } else if is_emoji(*c2) != is_emoji(*c1) {
            flush_buf(&mut text, &mut commands);
            commands.push(if is_emoji(*c2) {
                Command::Start(StyleType::Emoji)
            } else {
                Command::End(StyleType::Emoji)
            });
            text.push(*c2);
        } else {
            text.push(*c2);
        }
    }
    flush_buf(&mut text, &mut commands);

    parse_end(&mut commands, &chars, &just_added, &temp_style);

    commands
}

fn parse_end(
    commands: &mut Vec<Command>,
    chars: &[char],
    just_added: &Option<StyleType>,
    temp_style: &TextStyle,
) {
    // Last two characters
    let (Some(c1), Some(c2)) = (chars.get(chars.len() - 2), chars.last()) else {
        unreachable!();
    };
    if !c1.is_whitespace()
        && let Some(s) = StyleType::from_char(*c2)
        && !just_added.is_some_and(|j| j == s)
        && temp_style.get(s)
    {
        // println!("  last: {c2}");
        commands.push(Command::End(s));
    } else {
        match (is_emoji(*c1), is_emoji(*c2)) {
            (true, false) => commands.push(Command::End(StyleType::Emoji)),
            (false, true) => commands.push(Command::Start(StyleType::Emoji)),
            _ => {}
        }
        commands.push(Command::AddText(c2.to_string()));
        if is_emoji(*c2) {
            commands.push(Command::End(StyleType::Emoji));
        }
    };
}

fn parse_beginning(
    commands: &mut Vec<Command>,
    chars: &[char],
    just_added: &mut Option<StyleType>,
    temp_style: &mut TextStyle,
) -> bool {
    match (chars.first(), chars.get(1)) {
        (None, None) => true,
        (None, Some(_)) => true,
        (Some(c), None) => {
            if is_emoji(*c) {
                commands.push(Command::Start(StyleType::Emoji));
            }
            commands.push(Command::AddText(c.to_string()));
            if is_emoji(*c) {
                commands.push(Command::End(StyleType::Emoji));
            }
            true
        }
        (Some(c1), Some(c2)) => {
            let cmd = if !c2.is_whitespace()
                && let Some(s) = StyleType::from_char(*c1)
            {
                // "*hi"
                *just_added = Some(s);
                temp_style.apply(s, true);
                Command::Start(s)
            } else {
                if is_emoji(*c1) {
                    commands.push(Command::Start(StyleType::Emoji));
                }
                Command::AddText(c1.to_string())
            };
            commands.push(cmd);
            false
        }
    }
}

fn flush_buf(text: &mut String, cmds: &mut Vec<Command>) {
    if text.is_empty() {
        return;
    }
    cmds.push(Command::AddText(std::mem::take(text)));
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT: TextStyle = TextStyle {
        bold: false,
        italic: false,
        strikethrough: false,
        monospace: false,
        emoji: false,
    };

    const BOLD: TextStyle = TextStyle {
        bold: true,
        ..DEFAULT
    };
    const ITALIC: TextStyle = TextStyle {
        italic: true,
        ..DEFAULT
    };
    const STRIKETHROUGH: TextStyle = TextStyle {
        strikethrough: true,
        ..DEFAULT
    };
    const EMOJI: TextStyle = TextStyle {
        emoji: true,
        ..DEFAULT
    };

    const BOLD_ITALIC: TextStyle = TextStyle {
        bold: true,
        italic: true,
        ..DEFAULT
    };

    #[test]
    fn spans_formatting() {
        cmp_spans("*hello* world", &[("hello", BOLD), (" world", DEFAULT)]);
        cmp_spans(
            "*bold _italic* still italic_?",
            &[("bold _italic", BOLD), (" still italic_?", DEFAULT)],
        );

        cmp_spans(
            "*bold* _italic_ ~strike~",
            &[
                ("bold", BOLD),
                (" ", DEFAULT),
                ("italic", ITALIC),
                (" ", DEFAULT),
                ("strike", STRIKETHROUGH),
            ],
        );

        // adjacent markers
        cmp_spans(
            "*bold*_italic_~strike~",
            &[
                ("bold", BOLD),
                ("italic", ITALIC),
                ("strike", STRIKETHROUGH),
            ],
        );

        cmp_spans("*bold without end", &[("*bold without end", DEFAULT)]);
        cmp_spans("_italic without end", &[("_italic without end", DEFAULT)]);
        // nested (invalid) overlap
        cmp_spans(
            "*bold _italic* still italic_?",
            &[("bold _italic", BOLD), (" still italic_?", DEFAULT)],
        );
        // reverse overlap
        cmp_spans(
            "_italic *bold_ still bold*",
            &[("italic *bold", ITALIC), (" still bold*", DEFAULT)],
        );

        // markers inside words (should not format)
        cmp_spans("hel*lo*world", &[("hel*lo*world", DEFAULT)]);
        // whitespace inside markers (invalid)
        cmp_spans("* bold*", &[("* bold*", DEFAULT)]);
        cmp_spans("*bold *", &[("*bold *", DEFAULT)]);

        // punctuation adjacency
        cmp_spans(
            "*bold*, _italic_. ~strike~!",
            &[
                ("bold", BOLD),
                (", ", DEFAULT),
                ("italic", ITALIC),
                (". ", DEFAULT),
                ("strike", STRIKETHROUGH),
                ("!", DEFAULT),
            ],
        );

        cmp_spans(
            "*valid* _invalid *nested_ still*",
            &[
                ("valid", BOLD),
                (" ", DEFAULT),
                ("invalid *nested", ITALIC),
                (" still*", DEFAULT),
            ],
        );

        // newline handling (no multiline formatting)
        cmp_spans("*bold\nnot bold*", &[("*bold\nnot bold*", DEFAULT)]);
        // zero-width space (treated as normal char)
        cmp_spans("*bold\u{200B}text*", &[("bold\u{200B}text", BOLD)]);

        // repeated toggling
        cmp_spans("*this *is *a *mess*", &[("this *is *a *mess", BOLD)]);

        // lone markers
        cmp_spans(
            "this * is not formatting",
            &[("this * is not formatting", DEFAULT)],
        );
        cmp_spans("nor is this _", &[("nor is this _", DEFAULT)]);

        cmp_spans("~strike~test", &[("~strike~test", DEFAULT)]);

        // mixed tight formatting
        cmp_spans(
            "*a*_b_~c~",
            &[("a", BOLD), ("b", ITALIC), ("c", STRIKETHROUGH)],
        );

        cmp_spans(
            "(*bold*)",
            &[("(", DEFAULT), ("bold", BOLD), (")", DEFAULT)],
        );
        cmp_spans("***bold***", &[("**bold", BOLD), ("**", DEFAULT)]);
        cmp_spans("*_text_*", &[("text", BOLD_ITALIC)]);
        cmp_spans("_*text*_", &[("text", BOLD_ITALIC)]);
    }

    #[test]
    fn spans_repeated() {
        cmp_spans("****", &[("*", BOLD), ("*", DEFAULT)]);
        cmp_spans("** **", &[("* *", BOLD)]);
        cmp_spans("**", &[("**", DEFAULT)]);
        cmp_spans("***", &[("*", BOLD)]);
        cmp_spans("********", &[("*", BOLD), ("*", BOLD), ("**", DEFAULT)]);
    }

    #[test]
    fn spans_emojis() {
        cmp_spans(
            "some🙂text",
            &[("some", DEFAULT), ("🙂", EMOJI), ("text", DEFAULT)],
        );
        cmp_spans(
            "some more 🙂 text",
            &[("some more ", DEFAULT), ("🙂", EMOJI), (" text", DEFAULT)],
        );
        cmp_spans("🙂", &[("🙂", EMOJI)]);
        cmp_spans("hi🙂", &[("hi", DEFAULT), ("🙂", EMOJI)]);
        cmp_spans("🙂hello", &[("🙂", EMOJI), ("hello", DEFAULT)]);

        cmp_spans("🙂🙂", &[("🙂🙂", EMOJI)]);
        cmp_spans("a🙂🙂", &[("a", DEFAULT), ("🙂🙂", EMOJI)]);
        cmp_spans("a 🙂🙂", &[("a ", DEFAULT), ("🙂🙂", EMOJI)]);
        cmp_spans("🙂🙂s", &[("🙂🙂", EMOJI), ("s", DEFAULT)]);
        cmp_spans("🙂🙂 s", &[("🙂🙂", EMOJI), (" s", DEFAULT)]);
        cmp_spans("a🙂🙂s", &[("a", DEFAULT), ("🙂🙂", EMOJI), ("s", DEFAULT)]);
    }

    fn cmp_spans(input: &str, b: &[(&'static str, TextStyle)]) {
        let a = generate_spans(input);
        if a.len() != b.len() {
            panic!("input: {input:?}\n  expected: {b:?}\n  found: {a:?}");
        }
        for (a, b) in a.iter().zip(b) {
            if a.0 != b.0 || a.1 != b.1 {
                panic!("input: {input:?}\n  expected: {b:?}\n  found: {a:?}");
            }
        }
    }
}
