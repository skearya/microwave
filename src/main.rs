use std::time::Duration;

use iced::{
    alignment::Vertical,
    color,
    widget::{button, column, container, pick_list, radio, row, svg, text, text_input},
    Element, Length, Subscription, Theme,
};

struct Microwave {
    headset: &'static str,
    muted: bool,
    mode: MicMode,
    mic: &'static str,
}

impl Default for Microwave {
    fn default() -> Self {
        Self {
            headset: "Quest 3",
            muted: false,
            mode: MicMode::MuteAndUnmute,
            mic: "Oculus Virtual Audio Device",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicMode {
    MuteAndUnmute,
    PushToTalk,
}

#[derive(Debug, Clone)]
enum Message {
    MicMode(MicMode),
    MicSelected(&'static str),
    MuteToggle,
}

fn main() -> iced::Result {
    iced::application("Test", update, view)
        .window_size((450.0, 600.0))
        .theme(theme)
        .subscription(subscription)
        .run()
}

fn theme(_microwave: &Microwave) -> Theme {
    Theme::Light
}

fn subscription(_microwave: &Microwave) -> Subscription<Message> {
    iced::time::every(Duration::from_secs(1)).map(|_| Message::MuteToggle)
}

fn update(microwave: &mut Microwave, message: Message) {
    match message {
        Message::MicMode(choice) => microwave.mode = choice,
        Message::MicSelected(choice) => microwave.mic = choice,
        Message::MuteToggle => microwave.muted = !microwave.muted,
    }
}

fn view(microwave: &Microwave) -> Element<Message> {
    let header = row![
        text("Microwave").width(Length::Fill).size(24),
        text!("Connected to {}", microwave.headset)
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

    let mic_toggle = if let MicMode::MuteAndUnmute = microwave.mode {
        Some(
            button(
                row![
                    text(if microwave.muted { "Unmute" } else { "Mute" }).width(Length::Fill),
                    svg(if microwave.muted {
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
            .on_press(Message::MuteToggle),
        )
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
            ["Oculus Virtual Audio Device", "Laptop Microphone"],
            Some(microwave.mic),
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
