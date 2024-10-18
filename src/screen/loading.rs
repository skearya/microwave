use iced::{
    futures::channel::mpsc,
    widget::{container, text},
    Element, Length,
};

use super::{
    error::Error,
    ready::{MicMode, Ready},
};
use crate::{microphone, ovr, poller, State};

#[derive(Debug, Clone)]
pub enum Message {
    Ready((String, mpsc::Sender<poller::Message>)),
}

pub fn update(message: Message) -> Option<State> {
    let state = match message {
        Message::Ready((headset, poller)) => {
            match unsafe { microphone::active() } {
                Ok(mics) if !mics.is_empty() => State::Ready(Ready {
                    poller,
                    headset,
                    mic: mics
                        .iter()
                        .find(|mic| mic.name.contains("Headset Microphone"))
                        .unwrap_or_else(|| &mics[0])
                        .clone(),
                    mics: mics.into_iter().map(|mic| mic.name).collect(),
                    mode: MicMode::MuteAndUnmute,
                    binding: ovr::binding_to_string(1024 | 4 /* L Thumb + R Thumb */),
                    is_setting_binding: false,
                }),
                Ok(_) => State::Error(Error {
                    error: "No microphones found".to_string(),
                }),
                Err(error) => State::Error(Error {
                    error: error.to_string(),
                }),
            }
        }
    };

    Some(state)
}

pub fn view() -> Element<'static, Message> {
    container(text("Loading...")).center(Length::Fill).into()
}
