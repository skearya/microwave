mod microphone;
mod ovr;

use iced::{
    alignment::Vertical,
    color,
    widget::{button, column, container, pick_list, radio, row, svg, text, text_input},
    Element, Length, Subscription, Task, Theme,
};
use microphone::Microphone;
use ovr::Ovr;
use std::fmt::Write;
use std::time::Duration;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

struct Microwave {
    ovr: Ovr,
    mics: Vec<String>,
    mic: Option<Microphone>,
    mode: MicMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicMode {
    MuteAndUnmute,
    PushToTalk,
}

#[derive(Debug, Clone)]
enum Message {
    MicMode(MicMode),
    MicSelected(String),
    MuteToggle,
}

fn main() -> iced::Result {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).unwrap();
    }

    let ovr = unsafe { Ovr::new().expect("failed connecting to headset") };
    let session = ovr.session;

    let mics = unsafe { microphone::active().expect("error getting microphones") };

    iced::application("Test", update, view)
        .window_size((450.0, 600.0))
        .theme(theme)
        .subscription(subscription)
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

fn theme(_microwave: &Microwave) -> Theme {
    Theme::Light
}

fn subscription(microwave: &Microwave) -> Subscription<Message> {
    let interval = Duration::from_secs_f32(1000.0 / microwave.ovr.desc.DisplayRefreshRate / 1000.0);

    iced::time::every(interval).map(|_| Message::MuteToggle)
}

fn update(microwave: &mut Microwave, message: Message) {
    match message {
        Message::MicMode(choice) => microwave.mode = choice,
        Message::MicSelected(choice) => {
            let mics = unsafe { microphone::active().expect("error getting microphones") };

            microwave.mic = mics.iter().find(|mic| mic.name == choice).cloned();
        }
        Message::MuteToggle => {
            if let Some(mic) = &mut microwave.mic {
                let _ = unsafe { mic.set_mute(!mic.muted) };
            }
        }
    }
}

fn view(microwave: &Microwave) -> Element<Message> {
    let header = row![
        text("Microwave").width(Length::Fill).size(24),
        text!("Connected to {}", microwave.ovr.headset)
            .size(16)
            .color(color!(0x34C759))
    ]
    .align_y(Vertical::Center);

    let mic_mode = column![
        radio(
            "Mute / Unmute",
            MicMode::MuteAndUnmute,
            Some(microwave.mode),
            Message::MicMode,
        ),
        radio(
            "Push To Talk",
            MicMode::PushToTalk,
            Some(microwave.mode),
            Message::MicMode,
        )
    ]
    .spacing(8);

    let mic_toggle = if let Some(mic) = &microwave.mic {
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
        .style(button::secondary);

        if microwave.mode == MicMode::MuteAndUnmute {
            Some(button.on_press(Message::MuteToggle))
        } else {
            Some(button)
        }
    } else {
        None
    };

    let controller_binding = column![
        text("Controller Binding"),
        text_input("L Stick + R Stick", "").padding(16)
    ]
    .spacing(8);

    let mics = column![
        text("Microphone"),
        pick_list(
            microwave.mics.as_slice(),
            microwave.mic.as_ref().map(|mic| &mic.name),
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
