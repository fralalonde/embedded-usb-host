use crate::device::Device;
use crate::{AddressPool, DeviceDescriptor, Endpoint, RequestCode, RequestType, UsbError, WValue};

#[derive(Debug)]
#[derive(defmt::Format)]
pub enum HostEvent {
    Reset,
    Ready(Device, DeviceDescriptor),
    Tick,
}

/// Trait for host controller interface.
pub trait UsbHost {
    /// Perform endpoint upkeep, read / write operations
    fn update(&mut self, addr_pool: &mut AddressPool) -> Option<HostEvent>;

    /// Get the current connection max packet size
    /// This depends on negotiated USB link speed
    /// Endpoints may specify smaller packet sizes
    fn max_host_packet_size(&self) -> u16;

    /// Get current time in milliseconds
    /// The host holds the clock for all operations by drivers and the stack it belongs to
    fn now_ms(&self) -> u64;

    /// Issue a control transfer with an optional data stage to
    /// `ep`. The data stage direction is determined by the direction
    /// of `bm_request_type`.
    ///
    /// On success, the amount of data transferred into `buf` is returned.
    fn control_transfer(
        &mut self,
        ep: &mut dyn Endpoint,
        bm_request_type: RequestType,
        b_request: RequestCode,
        w_value: WValue,
        w_index: u16,
        buf: Option<&mut [u8]>,
    ) -> Result<usize, UsbError>;

    /// Issue a transfer from `ep` to the host.
    /// On success, the amount of data transferred into `buf` is returned.
    fn in_transfer(&mut self, ep: &mut dyn Endpoint, buf: &mut [u8]) -> Result<usize, UsbError>;

    /// Issue a transfer from the host to `ep`.
    /// On success, the amount of data transferred from `buf` is returned.
    /// This should always be equal to `buf.len()`.
    fn out_transfer(&mut self, ep: &mut dyn Endpoint, buf: &[u8]) -> Result<usize, UsbError>;
}