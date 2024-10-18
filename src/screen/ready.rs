use iced::{
    alignment::Vertical,
    color,
    futures::channel::mpsc,
    widget::{button, column, container, pick_list, radio, row, svg, text},
    Element, Length,
};

use super::error::Error;
use crate::{
    microphone::{self, Microphone},
    ovr::{self, ControllerEvent},
    poller, State,
};

#[derive(Debug)]
pub struct Ready {
    pub poller: mpsc::Sender<poller::Message>,
    pub headset: String,
    pub mic: Microphone,
    pub mics: Vec<String>,
    pub mode: MicMode,
    pub binding: String,
    pub is_setting_binding: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicMode {
    MuteAndUnmute,
    PushToTalk,
}

#[derive(Debug, Clone)]
pub enum Message {
    Controller(ovr::ControllerEvent),
    MuteToggle,
    MicMode(MicMode),
    MicSelected(String),
    SettingControllerBind,
}

impl Ready {
    pub fn update(&mut self, message: Message) -> Option<State> {
        match message {
            Message::Controller(event) => match (event, self.mode) {
                (ControllerEvent::Pressed, MicMode::PushToTalk) => {
                    let _ = unsafe { self.mic.set_mute(false) };
                }
                (ControllerEvent::Released, MicMode::PushToTalk) => {
                    let _ = unsafe { self.mic.set_mute(true) };
                }
                (ControllerEvent::Pressed, MicMode::MuteAndUnmute) => {
                    let _ = unsafe { self.mic.set_mute(!self.mic.muted) };
                }
                (ControllerEvent::Released, MicMode::MuteAndUnmute) => {}
                (ControllerEvent::BindingUpdate(binding), _) => {
                    self.binding = ovr::binding_to_string(binding)
                }
                (ControllerEvent::BindingSet(binding), _) => {
                    self.binding = ovr::binding_to_string(binding);
                    self.is_setting_binding = false;
                }
            },
            Message::MuteToggle => {
                let _ = unsafe { self.mic.set_mute(!self.mic.muted) };
            }
            Message::MicMode(mode) => {
                let mute = match mode {
                    MicMode::MuteAndUnmute => false,
                    MicMode::PushToTalk => true,
                };

                let _ = unsafe { self.mic.set_mute(mute) };

                self.mode = mode;
            }
            Message::MicSelected(choice) => {
                let mics = unsafe { microphone::active().expect("error getting microphones") };

                match mics.iter().find(|mic| mic.name == choice).cloned() {
                    Some(mic) => self.mic = mic,
                    None => {
                        return Some(State::Error(Error {
                            error: "Mic now unable to be used".to_string(),
                        }));
                    }
                }
            }
            Message::SettingControllerBind => {
                let _ = self.poller.try_send(poller::Message::SettingBind);

                self.is_setting_binding = true;
            }
        };

        None
    }

    pub fn view(&self) -> Element<Message> {
        let header = row![
            text("Microwave").width(Length::Fill).size(24),
            text!("Connected to {}", self.headset)
                .size(18)
                .color(color!(0x3FC661))
        ]
        .align_y(Vertical::Center);

        let mic_mode = column![
            radio(
                "Mute / Unmute",
                MicMode::MuteAndUnmute,
                Some(self.mode),
                Message::MicMode,
            ),
            radio(
                "Push To Talk",
                MicMode::PushToTalk,
                Some(self.mode),
                Message::MicMode,
            )
        ]
        .spacing(8);

        let mic_toggle = button(
            row![
                text(if self.mic.muted { "Unmute" } else { "Mute" }).width(Length::Fill),
                svg(if self.mic.muted {
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
        .on_press_maybe((self.mode == MicMode::MuteAndUnmute).then_some(Message::MuteToggle));

        let controller_binding = column![
            text("Controller Binding"),
            row![
                container(text(&self.binding))
                    .style(container::bordered_box)
                    .width(Length::Fill)
                    .padding(16),
                button("Set Bind")
                    .on_press_maybe(
                        (!self.is_setting_binding).then_some(Message::SettingControllerBind)
                    )
                    .padding(16)
            ]
            .spacing(8)
        ]
        .spacing(8);

        let mics = column![
            text("Microphone"),
            pick_list(
                self.mics.as_slice(),
                Some(&self.mic.name),
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
}
