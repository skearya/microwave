mod microphone;
mod ovr;

use iced::{
    alignment::Vertical,
    color,
    widget::{button, column, container, pick_list, radio, row, svg, text},
    Element, Length, Subscription, Task, Theme,
};
use microphone::Microphone;
use ovr::Ovr;
use std::time::Duration;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

struct Microwave {
    ovr: Ovr,
    mic: Option<Microphone>,
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
        let interval = Duration::from_secs_f32(1000.0 / self.ovr.desc.DisplayRefreshRate / 1000.0);

        iced::time::every(interval).map(|_| Message::PollControllers)
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::PollControllers => unsafe {
                if let Ok(pressed) = self.ovr.poll_input() {
                    if pressed {
                        self.update(Message::MuteToggle);
                    }
                }
            },
            Message::MuteToggle => {
                if let Some(mic) = &mut self.mic {
                    let _ = unsafe { mic.set_mute(!mic.muted) };
                }
            }
            Message::MicMode(choice) => self.mode = choice,
            Message::MicSelected(choice) => {
                let mics = unsafe { microphone::active().expect("error getting microphones") };

                self.mic = mics.iter().find(|mic| mic.name == choice).cloned();
            }
            Message::SettingControllerBind => {
                self.ovr.start_setting_bind();
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
            .on_press_maybe((self.mode == MicMode::MuteAndUnmute).then(|| Message::MuteToggle));

            Some(button)
        } else {
            None
        };

        let controller_binding = column![
            text("Controller Binding"),
            row![
                container(text(self.ovr.bind_to_string()))
                    .style(|theme| container::bordered_box(theme))
                    .width(Length::Fill)
                    .padding(16),
                button("Set Bind")
                    .on_press_maybe(
                        (!self.ovr.setting_bind).then(|| Message::SettingControllerBind)
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
}
