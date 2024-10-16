use windows::{
    core::GUID,
    Win32::{
        Devices::Properties,
        Media::Audio::{
            eCapture, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
            DEVICE_STATE_ACTIVE,
        },
        System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ},
        UI::Shell::PropertiesSystem::PROPERTYKEY,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Microphone {
    pub id: String,
    pub name: String,
    pub muted: bool,
    volume: IAudioEndpointVolume,
}

impl Microphone {
    pub unsafe fn set_mute(&mut self, mute: bool) -> windows::core::Result<()> {
        self.volume.SetMute(mute, &GUID::zeroed())?;
        self.muted = mute;

        Ok(())
    }
}

pub unsafe fn active() -> windows::core::Result<Vec<Microphone>> {
    let enumerator =
        CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let collection = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;

    let mut inputs = vec![];

    for i in 0..collection.GetCount()? {
        let device = collection.Item(i)?;
        let volume = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)?;

        inputs.push(Microphone {
            id: device.GetId()?.to_string()?,
            name: device
                .OpenPropertyStore(STGM_READ)?
                .GetValue(&PROPERTYKEY {
                    fmtid: Properties::DEVPKEY_Device_FriendlyName.fmtid,
                    pid: Properties::DEVPKEY_Device_FriendlyName.pid,
                })?
                .to_string(),
            muted: volume.GetMute()?.as_bool(),
            volume,
        });
    }

    Ok(inputs)
}
