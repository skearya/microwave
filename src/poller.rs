use std::time::Duration;

use iced::{
    futures::{channel::mpsc, SinkExt, Stream, StreamExt},
    stream,
};

use crate::ovr::{ControllerEvent, Ovr, OvrError, OVR_SESSION};

#[derive(Debug, Clone)]
pub enum Event {
    Ready(String, mpsc::Sender<Message>),
    Controller(ControllerEvent),
    Error(OvrError),
}

pub enum Message {
    SettingBind,
}

pub fn poll() -> impl Stream<Item = Event> {
    stream::channel(64, move |mut output| async move {
        let (sender, mut receiver) = mpsc::channel(64);

        let mut ovr = match unsafe { Ovr::new() } {
            Ok(ovr) => unsafe {
                OVR_SESSION = ovr.session;
                ovr
            },
            Err(error) => {
                let _ = output.send(Event::Error(error)).await;
                return;
            }
        };

        let interval = Duration::from_secs_f32(1000.0 / ovr.refresh_rate / 1000.0);

        let _ = output.send(Event::Ready(ovr.headset.clone(), sender)).await;

        loop {
            tokio::select! {
                message = receiver.next() => {
                    if let Some(Message::SettingBind) = message {
                        ovr.start_setting_binding();
                    }
                }
                () = tokio::time::sleep(interval) => {
                    match unsafe { ovr.poll_input() } {
                        Ok(Some(event)) => {
                            let _ = output.send(Event::Controller(event)).await;
                        },
                        Err(error) => {
                            let _ = output.send(Event::Error(error)).await;

                            unsafe {
                                Ovr::shutdown(OVR_SESSION);
                                OVR_SESSION = std::ptr::null_mut();
                            }

                            return;
                        },
                        _ => {}
                    }
                }
            };
        }
    })
}
