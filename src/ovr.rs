mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use bindings::{
    ovrButton__ovrButton_A, ovrButton__ovrButton_B, ovrButton__ovrButton_Enter,
    ovrButton__ovrButton_LThumb, ovrButton__ovrButton_RThumb, ovrButton__ovrButton_X,
    ovrButton__ovrButton_Y, ovrControllerType__ovrControllerType_Touch, ovrErrorInfo,
    ovrGraphicsLuid, ovrInitFlags__ovrInit_Invisible, ovrInitParams, ovrInputState, ovrResult,
    ovrSession, ovr_Create, ovr_Destroy, ovr_GetHmdDesc, ovr_GetInputState, ovr_GetLastErrorInfo,
    ovr_Initialize, ovr_Shutdown,
};

// I need to close the session when the window closes...
pub static mut OVR_SESSION: ovrSession = std::ptr::null_mut();

#[derive(Debug, Clone)]
pub struct OvrError {
    pub code: i32,
    pub reason: String,
}

pub type OvrResult<T = ()> = Result<T, OvrError>;

#[derive(Debug)]
pub struct Ovr {
    pub session: ovrSession,
    pub headset: String,
    pub refresh_rate: f32,
    pub binding: u32,
    pub setting_binding: bool,
    pressed: bool,
}

unsafe impl Send for Ovr {}
unsafe impl Sync for Ovr {}

#[derive(Debug, Clone)]
pub enum ControllerEvent {
    Pressed,
    Released,
    BindingUpdate(u32),
    BindingSet(u32),
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

        Ok(Self {
            session,
            headset: char_array_to_string(&desc.ProductName),
            refresh_rate: desc.DisplayRefreshRate,
            binding: ovrButton__ovrButton_LThumb as u32 | ovrButton__ovrButton_RThumb as u32,
            setting_binding: false,
            pressed: false,
        })
    }

    pub unsafe fn poll_input(&mut self) -> OvrResult<Option<ControllerEvent>> {
        let mut state: ovrInputState = std::mem::zeroed();
        ovr_GetInputState(
            self.session,
            ovrControllerType__ovrControllerType_Touch,
            &mut state,
        )
        .check()?;

        if self.setting_binding {
            let prev_binding = self.binding;
            self.binding |= state.Buttons;

            // Higher button value means more buttons pressed
            if prev_binding < self.binding {
                return Ok(Some(ControllerEvent::BindingUpdate(self.binding)));
            }

            // At least one button binded and they are no longer holding down the button(s)
            if self.binding != 0 && self.binding & state.Buttons == 0 {
                self.setting_binding = false;

                return Ok(Some(ControllerEvent::BindingSet(self.binding)));
            }

            return Ok(None);
        }

        let event = if state.Buttons & self.binding == self.binding {
            if !self.pressed {
                self.pressed = true;

                Some(ControllerEvent::Pressed)
            } else {
                None
            }
        } else if self.pressed {
            self.pressed = false;

            Some(ControllerEvent::Released)
        } else {
            None
        };

        Ok(event)
    }

    pub fn start_setting_binding(&mut self) {
        self.binding = 0;
        self.setting_binding = true;
    }

    pub unsafe fn shutdown(session: ovrSession) {
        ovr_Destroy(session);
        ovr_Shutdown();
    }
}

pub fn binding_to_string(binding: u32) -> String {
    let mut output = String::new();

    for (button, string) in MAPPINGS {
        if binding & button != 0 {
            if output.is_empty() {
                output.push_str(string);
            } else {
                output.push_str(" + ");
                output.push_str(string);
            }
        }
    }

    output
}

const MAPPINGS: &[(u32, &str)] = &[
    (ovrButton__ovrButton_A as u32, "A"),
    (ovrButton__ovrButton_B as u32, "B"),
    (ovrButton__ovrButton_X as u32, "X"),
    (ovrButton__ovrButton_Y as u32, "Y"),
    (ovrButton__ovrButton_LThumb as u32, "L Thumb"),
    (ovrButton__ovrButton_RThumb as u32, "R Thumb"),
    (ovrButton__ovrButton_Enter as u32, "Menu"),
];

trait OvrResultCheck {
    unsafe fn check(self) -> OvrResult;
}

impl OvrResultCheck for ovrResult {
    unsafe fn check(self) -> OvrResult {
        if self < 0 {
            let mut info: ovrErrorInfo = std::mem::zeroed();
            ovr_GetLastErrorInfo(&mut info);

            Err(OvrError {
                code: info.Result,
                reason: char_array_to_string(&info.ErrorString),
            })
        } else {
            Ok(())
        }
    }
}

fn char_array_to_string(input: &[i8]) -> String {
    String::from_utf8(input.iter().map(|&c| c as u8).filter(|&c| c != 0).collect())
        .unwrap_or("Unknown".to_string())
}
