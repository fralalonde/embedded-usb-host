//! Simple USB host-side driver for boot protocol keyboards.
use heapless::{FnvIndexMap, Vec};

use crate::{DevAddress, DescriptorParser, DescriptorRef, Device, DeviceDescriptor, Direction, Driver, Endpoint, InterfaceDescriptor, SingleEp, UsbError, UsbHost, EpAddress, map_entry_mut};
use embedded_midi::{MidiPorts, PacketParser, PortHandle, PortId, PortInfo};


use crate::audio::JackType;
use crate::class::audio::{AudioDescriptorRef};


// How long to wait before talking to the device again after setting
// its address. cf §9.2.6.3 of USB 2.0
// const SETTLE_DELAY: u64 = 2;

// How many total devices this driver can support.
const MAX_DEVICES: usize = 16;

// Max number of endpoints per device.
// 2 is the minimum for duplex devices
const MAX_ENDPOINTS_PER_DEV: usize = 2;

// Max number of jacks per endpoint
const MAX_JACKS_PER_ENDPOINT: usize = 4;

const MAX_ENDPOINTS: usize = MAX_DEVICES * MAX_ENDPOINTS_PER_DEV;

pub const USB_MIDI_PACKET_LEN: usize = 4;

pub const USB_CLASS_NONE: u8 = 0x00;
pub const USB_AUDIO_CLASS: u8 = 0x01;
pub const USB_AUDIO_CONTROL_SUBCLASS: u8 = 0x01;
pub const USB_MIDI_STREAMING_SUBCLASS: u8 = 0x03;

fn is_midi_interface(idesc: &InterfaceDescriptor) -> bool {
    idesc.b_interface_class == USB_AUDIO_CLASS
        && idesc.b_interface_sub_class == USB_MIDI_STREAMING_SUBCLASS
        && idesc.b_interface_protocol == 0x00
}

type JackId = u8;

pub struct UsbMidiDriver {
    /// Application MIDI ports registry
    with_midi: fn(&mut dyn FnMut(&mut (dyn MidiPorts + Send + Sync))),

    /// Keep track of endpoints for each device
    device_endpoints: FnvIndexMap<DevAddress, Vec<SingleEp, MAX_ENDPOINTS_PER_DEV>, MAX_DEVICES>,

    /// Keep track of jacks & ports for each endpoint
    ep_jack_port: FnvIndexMap<(DevAddress, EpAddress), FnvIndexMap<JackId, PortHandle, MAX_JACKS_PER_ENDPOINT>, MAX_ENDPOINTS>,

    next_port_id: usize,
}

impl UsbMidiDriver {
    pub fn new(midi_ports: fn(&mut dyn FnMut(&mut (dyn MidiPorts + Send + Sync)))) -> Self {
        UsbMidiDriver {
            with_midi: midi_ports,
            device_endpoints: FnvIndexMap::new(),
            ep_jack_port: FnvIndexMap::new(),
            next_port_id: 0,
        }
    }

    // fn add_device_buffer_handle(&mut self, addr: DevAddress, handle: PortHandle) -> Result<(), MidiError> {
    //     let handles = entry(&mut self.device_ports, addr, || Vec::new());
    //     if self.device_ports.contains_key(&addr) {
    //         unsafe {
    //             self.device_ports.get_mut(&addr)
    //                 .unwrap_unchecked()
    //                 .push(handle)
    //                 .or(Err(MidiError::TooManyJacks))
    //         }
    //     } else {
    //         let mut handles = Vec::new();
    //         handles.push(handle);
    //         self.device_ports.insert(addr, handles).or(Err(MidiError::TooManyDevices)).or(Err(MidiError::TooManyPorts));
    //         Ok(())
    //     }
    // }

    fn register_port(&mut self, dev_addr: DevAddress, ep_addr: EpAddress, jack_id: JackId) {
        let info = PortInfo {
            port_id: PortId::Usb(self.next_port_id),
            direction: ep_addr.direction().into(),
        };
        self.next_port_id += 1;
        (self.with_midi)(&mut move |midi: &mut (dyn MidiPorts + Send + Sync)| {
            match midi.acquire_port(info) {
                Ok(handle) => {
                    if let Some(jack_ports) = map_entry_mut(&mut self.ep_jack_port, (dev_addr, ep_addr), || FnvIndexMap::new()) {
                        jack_ports.insert(jack_id, handle);
                    } else {
                        warn!("TooManyEndpoints")
                    }
                }
                Err(err) => {
                    warn!("MIDI Ports error: {}", err)
                }
            }
        });
    }

    fn register_ep(&mut self, addr: DevAddress, new_ep: SingleEp) {
        if let Some(endpoints) = map_entry_mut(&mut self.device_endpoints, addr, || Vec::new()) {
            if !endpoints.contains(&new_ep) {
                debug!("Registered endpoint? {}", new_ep);
                if endpoints.push(new_ep).is_ok() {
                    // debug!("Registered endpoint? {}", new_ep)
                } else {
                    warn!("Too many endpoints")
                }
            } else {
                warn!("Duplicate endpoint? {}", new_ep)
            }
        } else {
            warn!("TooManyDevices")
        }
    }
}

impl From<Direction> for embedded_midi::PortDirection {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::Out => embedded_midi::PortDirection::Out,
            Direction::In => embedded_midi::PortDirection::In,
        }
    }
}

impl Driver for UsbMidiDriver {
    fn register(&mut self, host: &mut dyn UsbHost, device: &mut Device, _dev_desc: &DeviceDescriptor, parser: &mut DescriptorParser) -> Result<bool, UsbError> {
        let mut config = None;
        let mut midi_interface = None;
        let mut ep_in: Option<EpAddress> = None;
        let mut ep_out: Option<EpAddress> = None;
        let dev_count = self.device_endpoints.len();

        // phase 1 - identify interface and endpoints
        while let Some(desc) = parser.next() {
            match desc {
                DescriptorRef::Configuration(cdesc) => {
                    config = Some(cdesc)
                }

                DescriptorRef::Interface(idesc) => {
                    if is_midi_interface(idesc) {
                        if midi_interface.is_some() {
                            // new interface, done enumerating MIDI endpoints
                            warn!("Second MIDI interface found");
                            break;
                        }
                        midi_interface = Some(idesc)
                    }
                }

                DescriptorRef::Audio1Endpoint(edesc) => {
                    if let Some(_) = midi_interface {
                        let ep = device.audio1_endpoint(edesc)?;
                        let ep_addr = match ep.direction() {
                            Direction::Out => &mut ep_out,
                            Direction::In => &mut ep_in,
                        };
                        if ep_addr.is_some() {
                            warn!("More than one audio endpoint for device")
                        } else {
                            *ep_addr = Some(ep.endpoint_address());
                        }

                        self.register_ep(device.get_address(), ep);
                    }
                }
                _ => {
                    // debug!("USB Descriptor {:?}", desc);
                }
            }
        }

        // phase 2 - select device configuration & protocol
        if let Some(midi_if) = midi_interface {
            if let Some(cfg) = config {
                device.set_configuration(host, cfg.b_configuration_value)?;
                debug!("USB MIDI Device Configuration Set {}", cfg.b_configuration_value);
                host.wait_ms(10);
            } else {
                error!("USB MIDI Device not configured");
                return Ok(false);
            }

            // // TODO wait 10ms then set_interface
            // if let Err(e) = device.set_interface(host, midi_if.b_interface_number, midi_if.b_alternate_setting) {
            //     // should not matter? "Selecting a configuration, by default, also activates the first alternate setting in each interface in that configuration."
            //     warn!("USB MIDI Device set interface {}[{}] failed (ignored) {:?}", midi_if.b_interface_number,  midi_if.b_alternate_setting, e)
            // }
            // debug!("USB MIDI Device Interface Set {}[{}]",  midi_if.b_interface_number,  midi_if.b_alternate_setting);
        }

        // phase 3 - create ports for each jack
        parser.rewind();
        while let Some(desc) = parser.next() {
            match desc {
                DescriptorRef::Audio(AudioDescriptorRef::MSOutJack(out_jack)) => {
                    if out_jack.b_jack_type == JackType::Embedded as u8 {
                        if let Some(ep_out) = ep_out {
                            self.register_port(device.get_address(), ep_out, out_jack.b_jack_id)
                        } else {
                            warn!("Jack out of endpoint scope")
                        }
                    }
                }
                DescriptorRef::Audio(AudioDescriptorRef::MSInJack(in_jack)) => {
                    if in_jack.b_jack_type == JackType::Embedded as u8 {
                        if let Some(ep_in) = ep_in {
                            self.register_port(device.get_address(), ep_in, in_jack.b_jack_id)
                        } else {
                            warn!("Jack out of endpoint scope")
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(dev_count != self.device_endpoints.len())
    }


    fn unregister(&mut self, address: DevAddress) {
        if let Some(endpoints) = self.device_endpoints.remove(&address) {
            for ep in endpoints {
                if let Some(jack_handle) = self.ep_jack_port.remove(&(address, ep.endpoint_address())) {
                    for handle in jack_handle.values() {
                        (self.with_midi)(&mut |midi: &mut (dyn MidiPorts + Send + Sync)| midi.release_port(handle))
                    }
                }
            }
        }
    }

    fn tick(&mut self, host: &mut dyn UsbHost) -> Result<(), UsbError> {
        (self.with_midi)(&mut |midi: &mut (dyn MidiPorts + Send + Sync)| {
            for ep in self.device_endpoints.values_mut().flat_map(|eps| eps.iter_mut()) {
                if let Some(jack_port) = self.ep_jack_port.get_mut(&(ep.device_address(), ep.endpoint_address())) {
                    match ep.direction() {
                        Direction::Out => {
                            // sent packets are edited with the cable_num / jack_id of their MIDI port
                            for (jack_id, port_handle) in jack_port.iter() {
                                loop {
                                    // TODO send multiple packets at once
                                    match midi.read(port_handle) {
                                        Ok(None) => break,
                                        Ok(Some(mut packet)) => {
                                            packet.set_cable_number(*jack_id);
                                            if let Err(e) = host.out_transfer(ep, packet.bytes()) {
                                                warn!("USB OUT failed {:?}", e)
                                            }
                                        }
                                        Err(err) => {
                                            warn!("Failed to write to MIDI port: {}", err);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Direction::In => {
                            // received packets are dispatched to ports according to their cable_num / jack_id
                            let mut buf = [0; 64];

                            match host.in_transfer(ep, &mut buf) {
                                Ok(0) => {}
                                Ok(len) => {
                                    let mut pp = PacketParser::default();
                                    for b in &buf[..len] {
                                        match pp.advance(*b) {
                                            // TODO receive all packets at once
                                            Ok(Some(packet)) => {
                                                if let Some(port_handle) = jack_port.get(&packet.cable_number()) {
                                                    debug!("PACKET from jack {:?}", packet.cable_number() );
                                                    if let Err(err) = midi.write(port_handle, packet) {
                                                        warn!("Failed to read from MIDI port: {}", err);
                                                    }
                                                }
                                            }
                                            Err(e) => warn!("USB MIDI Packet Error{:?}", e),
                                            _ => {}
                                        }
                                    }
                                }
                                // Err(e) => warn!("USB MIDI IN Failed {:?}", e),
                                Err(e) => {}
                            }
                        }
                    }
                }
            }
        });


        Ok(())
    }
}
