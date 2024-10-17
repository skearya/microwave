mod microphone;
mod ovr;
mod poller;

use iced::{
    alignment::Vertical,
    color,
    futures::channel::mpsc,
    widget::{button, column, container, pick_list, radio, row, svg, text},
    Element, Length, Subscription, Task, Theme,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use microphone::Microphone;
use ovr::{ControllerEvent, Ovr, OvrError, OVR_SESSION};
use poller::Event;

#[derive(Debug)]
struct Microwave {
    state: State,
}

#[derive(Debug)]
enum State {
    Loading,
    Ready(Ready),
    Errored { error: String },
}

#[derive(Debug)]
struct Ready {
    poller: mpsc::Sender<poller::Message>,
    headset: String,
    mic: Microphone,
    mics: Vec<String>,
    mode: MicMode,
    binding: String,
    is_setting_binding: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicMode {
    MuteAndUnmute,
    PushToTalk,
}

#[derive(Debug, Clone)]
enum Message {
    Poller(poller::Event),
    MuteToggle,
    MicMode(MicMode),
    MicSelected(String),
    SettingControllerBind,
    Retry,
}

fn main() -> iced::Result {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).unwrap();
    }

    iced::application("Microwave", Microwave::update, Microwave::view)
        .window_size((450.0, 600.0))
        .theme(Microwave::theme)
        .subscription(Microwave::subscription)
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
        if let State::Errored { error: _ } = &self.state {
            Subscription::none()
        } else {
            Subscription::run(poller::poll).map(Message::Poller)
        }
    }

    fn update(&mut self, message: Message) {
        // TODO: Have messages be in an enum for each screen?
        let State::Ready(state) = &mut self.state else {
            match message {
                Message::Poller(Event::Ready(headset, poller)) => {
                    self.state = match unsafe { microphone::active() } {
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
                            binding: ovr::binding_to_string(1024 | 4),
                            is_setting_binding: false,
                        }),
                        Ok(_) => State::Errored {
                            error: "No microphones found".to_string(),
                        },
                        Err(error) => State::Errored {
                            error: error.to_string(),
                        },
                    };
                }
                Message::Poller(Event::Error(OvrError { code, reason })) => {
                    self.state = State::Errored {
                        error: format!("OVR Error\nCode {code}\nReason {reason}"),
                    };
                }
                Message::Retry => self.state = State::Loading,
                _ => {}
            }

            return;
        };

        match message {
            Message::Poller(event) => match event {
                Event::Ready(_headset, _poller) => { /* Already handled */ }
                Event::Controller(event) => match (event, state.mode) {
                    (ControllerEvent::Pressed, MicMode::PushToTalk) => {
                        let _ = unsafe { state.mic.set_mute(true) };
                    }
                    (ControllerEvent::Released, MicMode::PushToTalk) => {
                        let _ = unsafe { state.mic.set_mute(false) };
                    }
                    (ControllerEvent::Pressed, MicMode::MuteAndUnmute) => {
                        let _ = unsafe { state.mic.set_mute(!state.mic.muted) };
                    }
                    (ControllerEvent::Released, MicMode::MuteAndUnmute) => {}
                    (ControllerEvent::BindingUpdate(binding), _) => {
                        state.binding = ovr::binding_to_string(binding)
                    }
                    (ControllerEvent::BindingSet(binding), _) => {
                        state.binding = ovr::binding_to_string(binding);
                        state.is_setting_binding = false;
                    }
                },
                Event::Error(OvrError { code, reason }) => {
                    self.state = State::Errored {
                        error: format!("OVR Error\nCode {code}\nReason {reason}"),
                    };
                }
            },
            Message::MuteToggle => {
                let _ = unsafe { state.mic.set_mute(!state.mic.muted) };
            }
            Message::MicMode(choice) => {
                if choice == MicMode::PushToTalk {
                    let _ = unsafe { state.mic.set_mute(false) };
                }

                state.mode = choice;
            }
            Message::MicSelected(choice) => {
                let mics = unsafe { microphone::active().expect("error getting microphones") };

                match mics.iter().find(|mic| mic.name == choice).cloned() {
                    Some(mic) => state.mic = mic,
                    None => {
                        self.state = State::Errored {
                            error: "Mic now unable to be used".to_string(),
                        };
                    }
                }
            }
            Message::SettingControllerBind => {
                let _ = state.poller.try_send(poller::Message::SettingBind);

                state.is_setting_binding = true;
            }
            Message::Retry => { /* Already handled */ }
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.state {
            State::Loading => Microwave::loading(),
            State::Ready(ready) => Microwave::ready(ready),
            State::Errored { error } => Microwave::errored(error),
        }
    }

    fn loading() -> Element<'static, Message> {
        container(text("Loading...")).center(Length::Fill).into()
    }

    fn ready(state: &Ready) -> Element<Message> {
        let header = row![
            text("Microwave").width(Length::Fill).size(24),
            text!("Connected to {}", state.headset)
                .size(18)
                .color(color!(0x3FC661))
        ]
        .align_y(Vertical::Center);

        let mic_mode = column![
            radio(
                "Mute / Unmute",
                MicMode::MuteAndUnmute,
                Some(state.mode),
                Message::MicMode,
            ),
            radio(
                "Push To Talk",
                MicMode::PushToTalk,
                Some(state.mode),
                Message::MicMode,
            )
        ]
        .spacing(8);

        let mic_toggle = button(
            row![
                text(if state.mic.muted { "Unmute" } else { "Mute" }).width(Length::Fill),
                svg(if state.mic.muted {
                    "res/muted.svg"
                } else {
                    "res/unmuted.svg"
                })
                .width(40),
            ]
            .align_y(Vertical::Center),
        )
        .width(Length::Fill)
        .padding(16)
        .style(button::secondary)
        .on_press_maybe((state.mode == MicMode::MuteAndUnmute).then_some(Message::MuteToggle));

        let controller_binding = column![
            text("Controller Binding"),
            row![
                container(text(&state.binding))
                    .style(container::bordered_box)
                    .width(Length::Fill)
                    .padding(16),
                button("Set Bind")
                    .on_press_maybe(
                        (!state.is_setting_binding).then_some(Message::SettingControllerBind)
                    )
                    .padding(16)
            ]
            .spacing(8)
        ]
        .spacing(8);

        let mics = column![
            text("Microphone"),
            pick_list(
                state.mics.as_slice(),
                Some(&state.mic.name),
                Message::MicSelected,
            )
            .width(Length::Fill)
            .padding(16)
        ]
        .spacing(8);

        let column = column![header, mic_mode, mic_toggle, controller_binding, mics].spacing(20);

        container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([36, 16])
            .into()
    }

    fn errored(error: &str) -> Element<'_, Message> {
        container(
            column![
                text(error).style(text::danger),
                button("Retry").on_press(Message::Retry)
            ]
            .spacing(8),
        )
        .center(Length::Fill)
        .padding(20)
        .into()
    }
}
