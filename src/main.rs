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

struct Microwave {
    poller: Option<mpsc::Sender<poller::Message>>,
    error: Option<String>,
    headset: String,
    mic: Option<Microphone>,
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
}

fn main() -> iced::Result {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).unwrap();
    }

    let mics = unsafe { microphone::active().expect("error getting microphones") };

    iced::application("Microwave", Microwave::update, Microwave::view)
        .window_size((450.0, 600.0))
        .theme(Microwave::theme)
        .subscription(Microwave::subscription)
        .run_with(move || {
            (
                Microwave {
                    poller: None,
                    error: None,
                    headset: "Disconnected".to_string(),
                    mic: mics
                        .iter()
                        .find(|mic| mic.name.contains("Headset Microphone"))
                        .cloned(),
                    mics: mics.into_iter().map(|mic| mic.name).collect(),
                    mode: MicMode::MuteAndUnmute,
                    binding: ovr::binding_to_string(1024 | 4),
                    is_setting_binding: false,
                },
                Task::none(),
            )
        })?;

    unsafe {
        if !OVR_SESSION.is_null() {
            Ovr::shutdown(OVR_SESSION);
        }

        CoUninitialize();
    }

    Ok(())
}

impl Microwave {
    fn theme(&self) -> Theme {
        Theme::Light
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(poller::poll).map(Message::Poller)
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Poller(event) => match event {
                Event::Ready(headset, sender) => {
                    self.headset = headset;
                    self.poller = Some(sender)
                }
                Event::Controller(event) => match (event, self.mode) {
                    (ControllerEvent::Pressed, MicMode::PushToTalk) => {
                        if let Some(mic) = &mut self.mic {
                            unsafe {
                                let _ = mic.set_mute(true);
                            }
                        }
                    }
                    (ControllerEvent::Released, MicMode::PushToTalk) => {
                        if let Some(mic) = &mut self.mic {
                            unsafe {
                                let _ = mic.set_mute(false);
                            }
                        }
                    }
                    (ControllerEvent::Pressed, MicMode::MuteAndUnmute) => {
                        self.update(Message::MuteToggle);
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
                Event::Error(OvrError { code, reason }) => {
                    self.error = Some(format!("OVR Error\nCode {code}\nReason {reason}"))
                }
            },
            Message::MuteToggle => {
                if let Some(mic) = &mut self.mic {
                    unsafe {
                        let _ = mic.set_mute(!mic.muted);
                    }
                }
            }
            Message::MicMode(choice) => {
                if choice == MicMode::PushToTalk {
                    if let Some(mic) = &mut self.mic {
                        unsafe {
                            let _ = mic.set_mute(false);
                        }
                    }
                }

                self.mode = choice
            }
            Message::MicSelected(choice) => {
                let mics = unsafe { microphone::active().expect("error getting microphones") };

                self.mic = mics.iter().find(|mic| mic.name == choice).cloned();
            }
            Message::SettingControllerBind => {
                if let Some(runner) = &mut self.poller {
                    let _ = runner.try_send(poller::Message::SettingBind);

                    self.is_setting_binding = true;
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        if let Some(error) = &self.error {
            self.errored(error.clone())
        } else if self.poller.is_some() {
            self.normal()
        } else {
            self.loading()
        }
    }

    fn loading(&self) -> Element<Message> {
        container(text("Loading...")).center(Length::Fill).into()
    }

    fn normal(&self) -> Element<Message> {
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

        let mic_toggle = if let Some(mic) = &self.mic {
            let button = button(
                row![
                    text(if mic.muted { "Unmute" } else { "Mute" }).width(Length::Fill),
                    svg(if mic.muted {
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

            Some(button)
        } else {
            None
        };

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
                self.mic.as_ref().map(|mic| &mic.name),
                Message::MicSelected,
            )
            .width(Length::Fill)
            .padding(16)
        ]
        .spacing(8);

        let column = column![header, mic_mode]
            .push_maybe(mic_toggle)
            .push(controller_binding)
            .push(mics)
            .spacing(20);

        container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([36, 16])
            .into()
    }

    fn errored(&self, error: String) -> Element<Message> {
        container(text(error).style(text::danger))
            .center(Length::Fill)
            .padding(20)
            .into()
    }
}
