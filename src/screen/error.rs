use iced::{
    widget::{button, column, container, text},
    Element, Length,
};

use crate::State;

#[derive(Debug)]
pub struct Error {
    pub error: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Retry,
}

impl Error {
    pub fn update(&mut self, message: Message) -> Option<State> {
        match message {
            Message::Retry => Some(State::Loading),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                text(&self.error).style(text::danger),
                button("Retry").on_press(Message::Retry)
            ]
            .spacing(8),
        )
        .center(Length::Fill)
        .padding(20)
        .into()
    }
}
