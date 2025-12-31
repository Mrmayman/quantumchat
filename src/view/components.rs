use iced::widget;

use crate::{Element, Message, stylesheet::styles::Theme};

pub fn center<'a>(child: impl Into<Element<'a>>) -> widget::Container<'a, Message, Theme> {
    widget::center(child)
        .style(|_| widget::container::Style::default())
        .into()
}
