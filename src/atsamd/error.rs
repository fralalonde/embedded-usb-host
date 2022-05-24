use crate::UsbError;

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[allow(unused)]
pub(crate) enum PipeErr {
    ShortPacket,
    InvalidPipe,
    InvalidToken,
    InvalidRequest,
    Stall,
    TransferFail,
    PipeErr,
    Flow,
    HardwareTimeout,
    DataToggle,
    SoftwareTimeout,
    Other(&'static str),
}

impl From<&'static str> for PipeErr {
    fn from(v: &'static str) -> Self {
        Self::Other(v)
    }
}

impl From<PipeErr> for UsbError {
    fn from(v: PipeErr) -> Self {
        match v {
            PipeErr::TransferFail => Self::Transient("Transfer failed"),
            PipeErr::Flow => Self::Transient("Data flow"),
            PipeErr::DataToggle => Self::Transient("Data toggle"),

            PipeErr::ShortPacket => Self::Permanent("Short packet"),
            PipeErr::InvalidPipe => Self::Permanent("Invalid pipe"),
            PipeErr::InvalidToken => Self::Permanent("Invalid token"),
            PipeErr::Stall => Self::Permanent("Stall"),
            PipeErr::PipeErr => Self::Permanent("Pipe error"),
            PipeErr::HardwareTimeout => Self::Permanent("Hardware timeout"),
            PipeErr::SoftwareTimeout => Self::Permanent("Software timeout"),
            PipeErr::Other(s) => Self::Permanent(s),
            PipeErr::InvalidRequest => Self::Permanent("Invalid request"),
        }
    }
}
