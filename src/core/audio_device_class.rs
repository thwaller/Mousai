use gtk::glib;

#[derive(Debug, Clone, Copy, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "MsaiAudioDeviceClass")]
pub enum AudioDeviceClass {
    Source,
    Sink,
}

impl Default for AudioDeviceClass {
    fn default() -> Self {
        Self::Source
    }
}

impl AudioDeviceClass {
    pub fn for_str(string: &str) -> anyhow::Result<Self> {
        match string {
            "Audio/Source" => Ok(Self::Source),
            "Audio/Sink" => Ok(Self::Sink),
            unknown => Err(anyhow::anyhow!("Unknown device class `{unknown}`")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "Audio/Source",
            Self::Sink => "Audio/Sink",
        }
    }
}
