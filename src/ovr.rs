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
    pub button_binding: u32,
    pub trigger_binding: u8,
    pub is_setting_binding: bool,
    pressed: bool,
}

unsafe impl Send for Ovr {}
unsafe impl Sync for Ovr {}

#[derive(Debug, Clone)]
pub enum ControllerEvent {
    Pressed,
    Released,
    BindingUpdate(String),
    BindingSet(String),
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
            button_binding: ovrButton__ovrButton_LThumb as u32 | ovrButton__ovrButton_RThumb as u32,
            trigger_binding: 0,
            is_setting_binding: false,
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

        let triggers = [
            (L_INDEX_TRIGGER, &state.IndexTrigger[0]),
            (R_INDEX_TRIGGER, &state.IndexTrigger[1]),
            (L_HAND_TRIGGER, &state.HandTrigger[0]),
            (R_HAND_TRIGGER, &state.HandTrigger[1]),
        ];

        let mut trigger_state = 0;

        for (val, &trigger) in triggers {
            if trigger > 0.85 {
                trigger_state |= val;
            }
        }

        if self.is_setting_binding {
            let event = {
                let prev_button = self.button_binding;
                let prev_trigger = self.trigger_binding;

                self.button_binding |= state.Buttons;
                self.trigger_binding |= trigger_state;

                // Higher button value means more buttons pressed
                if prev_button < self.button_binding || prev_trigger < self.trigger_binding {
                    Some(ControllerEvent::BindingUpdate(self.binding_to_string()))
                } else {
                    let not_empty_bind = self.button_binding != 0 || self.trigger_binding != 0;
                    let not_pressing_bind = self.button_binding & state.Buttons == 0
                        && self.trigger_binding & trigger_state == 0;

                    if not_empty_bind && not_pressing_bind {
                        self.is_setting_binding = false;

                        Some(ControllerEvent::BindingSet(self.binding_to_string()))
                    } else {
                        None
                    }
                }
            };

            return Ok(event);
        }

        let holding_bind = state.Buttons & self.button_binding == self.button_binding
            && trigger_state & self.trigger_binding == self.trigger_binding;

        let event = if holding_bind {
            if self.pressed {
                None
            } else {
                self.pressed = true;

                Some(ControllerEvent::Pressed)
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
        self.button_binding = 0;
        self.trigger_binding = 0;
        self.is_setting_binding = true;
    }

    pub fn binding_to_string(&self) -> String {
        let mut output = String::new();

        for (trigger, string) in TRIGGER_MAPPINGS {
            if trigger & self.trigger_binding != 0 {
                if output.is_empty() {
                    output.push_str(string);
                } else {
                    output.push_str(" + ");
                    output.push_str(string);
                }
            }
        }

        for (button, string) in BUTTON_MAPPINGS {
            if button & self.button_binding != 0 {
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

    pub unsafe fn shutdown(session: ovrSession) {
        ovr_Destroy(session);
        ovr_Shutdown();
    }
}

const BUTTON_MAPPINGS: &[(u32, &str)] = &[
    (ovrButton__ovrButton_A as u32, "A"),
    (ovrButton__ovrButton_B as u32, "B"),
    (ovrButton__ovrButton_X as u32, "X"),
    (ovrButton__ovrButton_Y as u32, "Y"),
    (ovrButton__ovrButton_LThumb as u32, "L Thumb"),
    (ovrButton__ovrButton_RThumb as u32, "R Thumb"),
    (ovrButton__ovrButton_Enter as u32, "Menu"),
];

const L_INDEX_TRIGGER: u8 = 1 << 0;
const R_INDEX_TRIGGER: u8 = 1 << 1;
const L_HAND_TRIGGER: u8 = 1 << 2;
const R_HAND_TRIGGER: u8 = 1 << 3;

const TRIGGER_MAPPINGS: &[(u8, &str)] = &[
    (L_INDEX_TRIGGER, "L Index Trigger"),
    (R_INDEX_TRIGGER, "R Index Trigger"),
    (L_HAND_TRIGGER, "L Hand Trigger"),
    (R_HAND_TRIGGER, "R Hand Trigger"),
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
