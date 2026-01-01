use iced::{Length, widget};

use crate::{
    Element, Message,
    stylesheet::{color::Color, styles::Theme, widgets::StyleButton},
};

pub fn center<'a>(child: impl Into<Element<'a>>) -> widget::Container<'a, Message, Theme> {
    widget::center(child)
        .style(|_| widget::container::Style::default())
        .into()
}

pub fn sidebar_button<'a, A: PartialEq>(
    current: &A,
    selected: Option<&A>,
    text: impl Into<Element<'a>>,
    message: Message,
) -> Element<'a> {
    let is_selected = selected.is_some_and(|s| current == s);
    let button = widget::button(text)
        .on_press_maybe((!is_selected).then_some(message))
        .style(|n: &Theme, status| n.style_button(status, StyleButton::FlatExtraDark))
        .width(Length::Fill);

    underline_maybe(button, Color::SecondDark, !is_selected)
}

pub fn tsubtitle(t: &Theme) -> widget::text::Style {
    t.style_text(Color::SecondLight)
}

pub fn underline_maybe<'a>(e: impl Into<Element<'a>>, color: Color, un: bool) -> Element<'a> {
    if un {
        underline(e, color).into()
    } else {
        e.into()
    }
}

pub fn underline<'a>(e: impl Into<Element<'a>>, color: Color) -> widget::Stack<'a, Message, Theme> {
    widget::stack!(
        widget::column![e.into()],
        widget::column![
            widget::vertical_space(),
            widget::horizontal_rule(1).style(move |t: &Theme| t.style_rule(color, 1)),
            widget::Space::with_height(1),
        ]
    )
}

pub fn sbox<'a>(
    view: impl Into<Element<'a>>,
    color: Color,
) -> widget::Container<'a, Message, Theme> {
    widget::container(view).style(move |t: &Theme| t.style_container_sharp_box(0.0, color))
}
