mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::fmt::Display;

use bindings::{
    ovrButton__ovrButton_A, ovrButton__ovrButton_B, ovrButton__ovrButton_Enter,
    ovrButton__ovrButton_LThumb, ovrButton__ovrButton_RThumb, ovrButton__ovrButton_X,
    ovrButton__ovrButton_Y, ovrControllerType__ovrControllerType_Touch, ovrErrorInfo,
    ovrGraphicsLuid, ovrHmdDesc, ovrInitFlags__ovrInit_Invisible, ovrInitParams, ovrInputState,
    ovrResult, ovrSession, ovr_Create, ovr_Destroy, ovr_GetHmdDesc, ovr_GetInputState,
    ovr_GetLastErrorInfo, ovr_Initialize, ovr_Shutdown,
};

pub type OvrResult<T = ()> = Result<T, Box<ovrErrorInfo>>;

pub struct Ovr {
    pub headset: String,
    pub desc: ovrHmdDesc,
    pub binding: Vec<ControllerButtons>,
    pub held: Vec<ControllerButtons>,
    pub session: ovrSession,
}

impl Ovr {
    pub unsafe fn new() -> OvrResult<Self> {
        let params = ovrInitParams {
            Flags: ovrInitFlags__ovrInit_Invisible as u32,
            RequestedMinorVersion: 0,
            LogCallback: None,
            UserData: 0,
            ConnectionTimeoutMS: 0,
            pad0: std::mem::zeroed(),
        };

        ovr_Initialize(&params).check()?;

        let mut session: ovrSession = std::mem::zeroed();
        let mut luid: ovrGraphicsLuid = std::mem::zeroed();

        ovr_Create(&mut session, &mut luid).check()?;

        let desc = ovr_GetHmdDesc(session);
        let headset = String::from_utf8(desc.ProductName.iter().map(|&c| c as u8).collect())
            .unwrap_or("Unknown".to_string());

        Ok(Self {
            desc,
            headset,
            binding: vec![],
            held: vec![],
            session,
        })
    }

    pub unsafe fn poll_input(&mut self) -> OvrResult {
        let mut state: ovrInputState = std::mem::zeroed();

        ovr_GetInputState(
            self.session,
            ovrControllerType__ovrControllerType_Touch,
            &mut state,
        )
        .check()?;

        for (ovr_button, button) in MAPPINGS {
            if state.Buttons & ovr_button != 0 {
                self.held.push(*button);
            }
        }

        Ok(())
    }

    pub unsafe fn shutdown(session: ovrSession) {
        ovr_Destroy(session);
        ovr_Shutdown();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerButtons {
    A,
    B,
    X,
    Y,
    LThumb,
    RThumb,
    Menu,
}

impl Display for ControllerButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControllerButtons::A => write!(f, "A"),
            ControllerButtons::B => write!(f, "B"),
            ControllerButtons::X => write!(f, "X"),
            ControllerButtons::Y => write!(f, "Y"),
            ControllerButtons::LThumb => write!(f, "L Thumb"),
            ControllerButtons::RThumb => write!(f, "R Thumb"),
            ControllerButtons::Menu => write!(f, "Menu"),
        }
    }
}

#[rustfmt::skip]
const MAPPINGS: &[(u32, ControllerButtons)] = &[
    (ovrButton__ovrButton_A as u32, ControllerButtons::A),
    (ovrButton__ovrButton_B as u32, ControllerButtons::B),
    (ovrButton__ovrButton_X as u32, ControllerButtons::X),
    (ovrButton__ovrButton_Y as u32, ControllerButtons::Y),
    (ovrButton__ovrButton_LThumb as u32, ControllerButtons::LThumb),
    (ovrButton__ovrButton_RThumb as u32, ControllerButtons::RThumb),
    (ovrButton__ovrButton_Enter as u32, ControllerButtons::Menu),
];

trait OvrResultCheck {
    unsafe fn check(self) -> OvrResult;
}

impl OvrResultCheck for ovrResult {
    unsafe fn check(self) -> OvrResult {
        if self < 0 {
            let mut info: Box<ovrErrorInfo> = Box::new(std::mem::zeroed());
            ovr_GetLastErrorInfo(&mut *info);

            Err(info)
        } else {
            Ok(())
        }
    }
}
