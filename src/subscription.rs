use std::time::Duration;

use iced::{
    futures::{channel::mpsc, SinkExt, Stream, StreamExt},
    stream,
};

use crate::ovr::{ControllerEvent, Ovr, OvrError};

pub enum Event {
    Ready(mpsc::Sender<Message>),
    ControllerEvent(ControllerEvent),
    Error(OvrError),
}

pub enum Message {
    SettingBind,
}

pub fn poll(ovr: Ovr) -> impl Stream<Item = Event> {
    stream::channel(64, move |mut output| async move {
        let (sender, mut receiver) = mpsc::channel(64);

        let mut ovr = ovr;
        let interval = Duration::from_secs_f32(1000.0 / ovr.refresh_rate / 1000.0);

        let _ = output.send(Event::Ready(sender)).await;

        loop {
            tokio::select! {
                message = receiver.next() => {
                    if let Some(Message::SettingBind) = message {
                        ovr.start_setting_binding();
                    }
                }
                _ = tokio::time::sleep(interval) => {
                    match unsafe { ovr.poll_input() } {
                        Ok(Some(event)) => {
                            output.send(Event::ControllerEvent(event)).await;
                        },
                        Err(error) => {
                            output.send(Event::Error(error)).await;
                        },
                        _ => {}
                    }
                }
            };
        }
    })
}
