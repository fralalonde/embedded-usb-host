use heapless::{FnvIndexMap, Vec};

use crate::{DevAddress, DescriptorParser, DescriptorRef, Device, Direction, Driver, InterfaceDescriptor, Endpoint, UsbError, UsbHost, EpAddress, map_entry_mut, MaxPacketSize, EndpointProperties, ConfigNum, InterfaceNum, EpProps};
use embedded_midi::{MidiPorts, PacketParser, PortHandle, PortId, PortInfo};


use crate::audio::JackType;
use crate::class::audio::{AudioDescriptorRef};


// How long to wait before talking to the device again after setting
// its address. cf §9.2.6.3 of USB 2.0
// const SETTLE_DELAY: u64 = 2;

// How many total devices this driver can support.
const MAX_MIDI_DEVICES: usize = 16;

// Max number of endpoints per device.
// 2 is the minimum for duplex devices
const MAX_ENDPOINTS_PER_DEV: usize = 2;

// Max number of jacks per endpoint
const MAX_JACKS_PER_ENDPOINT: usize = 4;

const MAX_ENDPOINTS: usize = MAX_MIDI_DEVICES * MAX_ENDPOINTS_PER_DEV;

pub const USB_MIDI_PACKET_LEN: usize = 4;

pub const USB_CLASS_NONE: u8 = 0x00;
pub const USB_AUDIO_CLASS: u8 = 0x01;
pub const USB_AUDIO_CONTROL_SUBCLASS: u8 = 0x01;
pub const USB_MIDI_STREAMING_SUBCLASS: u8 = 0x03;

fn is_midi_interface(idesc: &InterfaceDescriptor) -> bool {
    idesc.b_interface_class == USB_AUDIO_CLASS
        && idesc.b_interface_sub_class == USB_MIDI_STREAMING_SUBCLASS
}

type JackId = u8;

pub struct UsbMidiDriver {
    /// Application MIDI ports registry
    with_midi: fn(&mut dyn FnMut(&mut (dyn MidiPorts + Send + Sync))),

    /// Keep track of endpoints for each device
    device_endpoints: FnvIndexMap<DevAddress, Vec<Endpoint, MAX_ENDPOINTS_PER_DEV>, MAX_MIDI_DEVICES>,

    /// Keep track of jacks & ports for each endpoint
    ep_jack_port: FnvIndexMap<EpProps, FnvIndexMap<JackId, PortHandle, MAX_JACKS_PER_ENDPOINT>, MAX_ENDPOINTS>,

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

    fn register_port(&mut self, ep: &EpProps, jack_id: JackId) {
        let info = PortInfo {
            port_id: PortId::Usb(self.next_port_id),
            direction: ep.direction().into(),
        };
        self.next_port_id += 1;
        (self.with_midi)(&mut move |midi: &mut (dyn MidiPorts + Send + Sync)| {
            match midi.acquire_port(info) {
                Ok(handle) => {
                    if let Some(jack_ports) = map_entry_mut(&mut self.ep_jack_port, *ep, || FnvIndexMap::new()) {
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
    fn accept(&self, _device: &mut Device, parser: &mut DescriptorParser) -> Option<(ConfigNum, InterfaceNum)> {
        let mut config = None;
        let mut midi_interface = None;

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
                _ => {}
            }
        }

        if let Some(iface) = midi_interface {
            if let Some(cfg) = config {
                return Some((cfg.b_configuration_value, iface.b_interface_number));
            }
        }
        None
    }

    fn register(&mut self, device: &mut Device, parser: &mut DescriptorParser) -> Result<(), UsbError> {
        let mut ep_in: Option<EpProps> = None;
        let mut ep_out: Option<EpProps> = None;

        let dev_addr = device.device_address();

        let mut register_ep = |dev_addr, max_packet_size: u16, b_endpoint_address: u8, bm_attributes: u8| {
            let new_ep = Endpoint::from_raw(dev_addr, max_packet_size, b_endpoint_address, bm_attributes);
            if let Some(prev_ep) = match new_ep.direction() {
                Direction::Out => ep_out.replace(new_ep.props().clone()),
                Direction::In => ep_in.replace(new_ep.props().clone()),
            } {
                warn!("More than one endpoint for device {}", prev_ep)
            }
            if let Some(endpoints) = map_entry_mut(&mut self.device_endpoints, dev_addr, || Vec::new()) {
                if !endpoints.push(new_ep).is_ok() {
                    warn!("Too many endpoints for device")
                }
            } else {
                warn!("TooManyDevices")
            }
        };

        // phase 1 - identify interface and endpoints
        while let Some(desc) = parser.next() {
            match desc {
                DescriptorRef::Endpoint(edesc) =>
                    register_ep(dev_addr, edesc.max_packet_size(), edesc.b_endpoint_address, edesc.bm_attributes),
                DescriptorRef::Audio1Endpoint(edesc) =>
                    register_ep(dev_addr, edesc.max_packet_size(), edesc.b_endpoint_address, edesc.bm_attributes),
                _ => {}
            }
        }

        // phase 2 - create ports for each jack
        parser.rewind();
        while let Some(desc) = parser.next() {
            match desc {
                DescriptorRef::Audio(AudioDescriptorRef::MSOutJack(out_jack)) => {
                    if out_jack.b_jack_type == JackType::Embedded as u8 {
                        if let Some(ep_out) = ep_out {
                            self.register_port(&ep_out, out_jack.b_jack_id)
                        }
                    }
                }
                DescriptorRef::Audio(AudioDescriptorRef::MSInJack(in_jack)) => {
                    if in_jack.b_jack_type == JackType::Embedded as u8 {
                        if let Some(ep_in) = ep_in {
                            self.register_port(&ep_in, in_jack.b_jack_id)
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }


    fn unregister(&mut self, address: DevAddress) {
        if let Some(endpoints) = self.device_endpoints.remove(&address) {
            for ep in endpoints {
                if let Some(jack_handle) = self.ep_jack_port.remove(ep.props()) {
                    for handle in jack_handle.values() {
                        (self.with_midi)(&mut |midi: &mut (dyn MidiPorts + Send + Sync)| midi.release_port(handle))
                    }
                }
            }
        }
    }

    fn run(&mut self, host: &mut dyn UsbHost, device: &mut Device) -> Result<(), UsbError> {
        (self.with_midi)(&mut |midi: &mut (dyn MidiPorts + Send + Sync)| {
            for ep in self.device_endpoints.get_mut(&device.device_address()).iter_mut().flat_map(|eps| eps.iter_mut()) {
                if let Some(jack_port) = self.ep_jack_port.get_mut(ep.props()) {
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
                                Err(_e) => {
                                    // warn!("USB MIDI IN Failed {:?}", e)
                                }
                            }
                        }
                    }
                }
            }
        });


        Ok(())
    }
}


// /* Setup for well known vendor/device specific configuration */
// void USBH_MIDI::setupDeviceSpecific()
// {
//         // Novation
//         if( vid == 0x1235 ) {
//                 // LaunchPad and LaunchKey endpoint attribute is interrupt
//                 // https://github.com/YuuichiAkagawa/USBH_MIDI/wiki/Novation-USB-Product-ID-List
//
//                 // LaunchPad: 0x20:S, 0x36:Mini, 0x51:Pro, 0x69:MK2
//                 if( pid == 0x20 || pid == 0x36 || pid == 0x51 || pid == 0x69 ) {
//                         bTransferTypeMask = 2;
//                         return;
//                 }
//
//                 // LaunchKey: 0x30-32,  0x35:Mini, 0x7B-0x7D:MK2, 0x0102,0x113-0x122:MiniMk3, 0x134-0x137:MK3
//                 if( (0x30 <= pid && pid <= 0x32) || pid == 0x35 || (0x7B <= pid && pid <= 0x7D)
//                   || pid == 0x102 || (0x113 <= pid && pid <= 0x122) || (0x134 <= pid && pid <= 0x137) ) {
//                         bTransferTypeMask = 2;
//                         return;
//                 }
//         }
// }