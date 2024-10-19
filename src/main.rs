#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod microphone;
mod ovr;
mod poller;
mod screen;

use iced::{
    window::{icon, Settings},
    Element, Subscription, Task, Theme,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use ovr::{Ovr, OvrError, OVR_SESSION};
use poller::Event;
use screen::{
    error::{self, Error},
    loading,
    ready::{self, Ready},
};

struct Microwave {
    state: State,
}

enum State {
    Loading,
    Ready(Ready),
    Error(Error),
}

#[derive(Debug, Clone)]
enum Message {
    Errored(String),
    Loading(loading::Message),
    Ready(ready::Message),
    Error(error::Message),
}

fn main() -> iced::Result {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).unwrap();
    }

    iced::application("Microwave", Microwave::update, Microwave::view)
        .theme(Microwave::theme)
        .subscription(Microwave::subscription)
        .window(Settings {
            size: (450.0, 600.0).into(),
            icon: icon::from_file_data(include_bytes!("../res/microwave.png"), None).ok(),
            ..Default::default()
        })
        .run_with(Microwave::new)?;

    // TODO: Probably replaceable with an on application close callback
    unsafe {
        if !OVR_SESSION.is_null() {
            Ovr::shutdown(OVR_SESSION);
        }

        CoUninitialize();
    }

    Ok(())
}

impl Microwave {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                state: State::Loading,
            },
            Task::none(),
        )
    }

    fn theme(&self) -> Theme {
        Theme::Light
    }

    fn subscription(&self) -> Subscription<Message> {
        if let State::Error(_) = &self.state {
            Subscription::none()
        } else {
            Subscription::run(poller::poll).map(|event| match event {
                Event::Ready(headset, sender) => {
                    Message::Loading(loading::Message::Ready((headset, sender)))
                }
                Event::Controller(event) => Message::Ready(ready::Message::Controller(event)),
                Event::Error(OvrError { code, reason }) => {
                    Message::Errored(format!("OVR Error\nCode {code}\nReason {reason}"))
                }
            })
        }
    }

    fn update(&mut self, message: Message) {
        let update = match message {
            Message::Errored(message) => Some(State::Error(Error { error: message })),
            Message::Loading(message) => {
                let State::Loading = self.state else { return };

                Some(loading::update(message))
            }
            Message::Ready(message) => {
                let State::Ready(ready) = &mut self.state else {
                    return;
                };

                ready.update(message)
            }
            Message::Error(message) => {
                let State::Error(error) = &mut self.state else {
                    return;
                };

                Some(error.update(message))
            }
        };

        if let Some(state) = update {
            self.state = state;
        };
    }

    fn view(&self) -> Element<Message> {
        match &self.state {
            State::Loading => loading::view().map(Message::Loading),
            State::Ready(ready) => ready.view().map(Message::Ready),
            State::Error(error) => error.view().map(Message::Error),
        }
    }
}
