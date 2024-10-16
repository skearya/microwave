mod microphone;
mod ovr;
mod subscription;

use iced::{
    alignment::Vertical,
    color,
    widget::{button, column, container, pick_list, radio, row, svg, text},
    Element, Length, Subscription, Task, Theme,
};
use std::time::Duration;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use microphone::Microphone;
use ovr::{ControllerEvent, Ovr};

struct Microwave {
    binding: String,
    setting_binding: bool,
    mic: Microphone,
    mics: Vec<String>,
    mode: MicMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicMode {
    MuteAndUnmute,
    PushToTalk,
}

#[derive(Debug, Clone)]
enum Message {
    PollControllers,
    MuteToggle,
    MicMode(MicMode),
    MicSelected(String),
    SettingControllerBind,
}

fn main() -> iced::Result {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).unwrap();
    }

    let ovr = unsafe { Ovr::new().expect("failed connecting to headset") };
    let session = ovr.session;

    let mics = unsafe { microphone::active().expect("error getting microphones") };

    iced::application("Test", Microwave::update, Microwave::view)
        .window_size((450.0, 600.0))
        .theme(Microwave::theme)
        .subscription(Microwave::subscription)
        .run_with(move || {
            (
                Microwave {
                    ovr,
                    binding: ovr::binding_to_string(1024 | 4),
                    setting_binding: false,
                    mic: mics
                        .iter()
                        .find(|mic| mic.name.contains("Headset Microphone"))
                        .cloned(),
                    mics: mics.into_iter().map(|mic| mic.name).collect(),
                    mode: MicMode::MuteAndUnmute,
                },
                Task::none(),
            )
        })?;

    unsafe {
        Ovr::shutdown(session);
        CoUninitialize();
    }

    Ok(())
}

impl Microwave {
    fn theme(&self) -> Theme {
        Theme::Light
    }

    fn subscription(&self) -> Subscription<Message> {
        let interval = Duration::from_secs_f32(1000.0 / self.ovr.refresh_rate / 1000.0);

        iced::time::every(interval).map(|_| Message::PollControllers)
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::PollControllers => {
                if let Ok(Some(event)) = unsafe { self.ovr.poll_input() } {
                    match (event, self.mode) {
                        (ControllerEvent::Pressed, MicMode::PushToTalk) => unsafe {
                            let _ = self.mic.set_mute(true);
                        },
                        (ControllerEvent::Released, MicMode::PushToTalk) => unsafe {
                            let _ = self.mic.set_mute(false);
                        },
                        (ControllerEvent::Pressed, MicMode::MuteAndUnmute) => {
                            self.update(Message::MuteToggle);
                        }
                        (ControllerEvent::Released, MicMode::MuteAndUnmute) => {}
                        (ControllerEvent::BindingUpdate(binding), _) => {
                            self.binding = ovr::binding_to_string(binding)
                        }
                        (ControllerEvent::BindingSet(binding), _) => {
                            self.binding = ovr::binding_to_string(binding);
                            self.setting_binding = false;
                        }
                    }
                }
            }
            Message::MuteToggle => {
                let _ = unsafe { self.mic.set_mute(!self.mic.muted) };
            }
            Message::MicMode(choice) => {
                if choice == MicMode::PushToTalk {}

                self.mode = choice
            }
            Message::MicSelected(choice) => {
                let mics = unsafe { microphone::active().expect("error getting microphones") };

                self.mic = mics.iter().find(|mic| mic.name == choice).cloned();
            }
            Message::SettingControllerBind => {
                self.ovr.start_setting_binding();
                self.setting_binding = true;
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let header = row![
            text("Microwave").width(Length::Fill).size(24),
            text!("Connected to {}", self.ovr.headset)
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
                        (!self.setting_binding).then_some(Message::SettingControllerBind)
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
