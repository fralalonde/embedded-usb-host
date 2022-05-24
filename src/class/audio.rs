#![allow(dead_code)]

use crate::class::audio::AudioDescriptorRef::Unknown;
use crate::DescriptorType;

#[derive(Clone, Copy, Debug, PartialEq, defmt::Format, strum_macros::FromRepr)]
#[repr(u8)]
pub enum AudioSubclass {
    AudioControl = 0x01,
    AudioStream = 0x02,
    MidiStream = 0x03,
}

#[derive(Debug, defmt::Format)]
pub enum AudioDescriptorRef<'a> {
    ACInterfaceHeader(&'a ACInterfaceHeaderDescriptor),
    ACClockSource(&'a ACClockSourceDescriptor),
    ACClockSelector(&'a ACClockSelectorDescriptor),
    ACFeatureUnit(&'a ACFeatureUnitDescriptor),
    ACInputTerminal(&'a ACInputTerminalDescriptor),
    ACOutputTerminal(&'a ACOutputTerminalDescriptor),

    ASInterface(&'a ASInterfaceDescriptor),
    ASFormatType1(&'a ASFormatType1Descriptor),

    MSInterface(&'a MSInterfaceDescriptor),
    MSInJack(&'a MSInJackDescriptor),
    MSOutJack(&'a MSOutJackDescriptor),

    ASEndpoint(&'a ASEndpointDescriptor),
    MSEndpoint(&'a MSEndpointDescriptor),

    Unknown(&'a [u8]),
}

pub fn parse(subclass: Option<u8>, desc_type: DescriptorType, buf: &[u8]) -> AudioDescriptorRef {
    if let Some(subclass) = subclass {
        if buf.len() < 3 {
            return Unknown(buf);
        }
        if let Some(subclass) = AudioSubclass::from_repr(subclass) {
            return match desc_type {
                DescriptorType::ClassInterface => match subclass {
                    AudioSubclass::AudioControl => match ACInterfaceSubtype::from_repr(buf[2]) {
                        Some(ACInterfaceSubtype::InterfaceHeader) => {
                            AudioDescriptorRef::ACInterfaceHeader(unsafe {
                                &*(buf.as_ptr() as *const _)
                            })
                        }
                        Some(ACInterfaceSubtype::InputTerminalDescriptor) => {
                            AudioDescriptorRef::ACInputTerminal(unsafe {
                                &*(buf.as_ptr() as *const _)
                            })
                        }
                        Some(ACInterfaceSubtype::OutputTerminalDescriptor) => {
                            AudioDescriptorRef::ACOutputTerminal(unsafe {
                                &*(buf.as_ptr() as *const _)
                            })
                        }
                        Some(ACInterfaceSubtype::FeatureUnitDescriptor) => {
                            AudioDescriptorRef::ACFeatureUnit(unsafe {
                                &*(buf.as_ptr() as *const _)
                            })
                        }
                        Some(ACInterfaceSubtype::ClockSourceDescriptor) => {
                            AudioDescriptorRef::ACClockSource(unsafe {
                                &*(buf.as_ptr() as *const _)
                            })
                        }
                        Some(ACInterfaceSubtype::ClockSelectorDescriptor) => {
                            AudioDescriptorRef::ACClockSelector(unsafe {
                                &*(buf.as_ptr() as *const _)
                            })
                        }
                        _ => Unknown(buf),
                    },
                    AudioSubclass::AudioStream => match ASInterfaceSubtype::from_repr(buf[2]) {
                        Some(ASInterfaceSubtype::AudioStreamHeader) => {
                            AudioDescriptorRef::ASInterface(unsafe { &*(buf.as_ptr() as *const _) })
                        }
                        Some(ASInterfaceSubtype::FormatType1) => {
                            AudioDescriptorRef::ASFormatType1(unsafe {
                                &*(buf.as_ptr() as *const _)
                            })
                        }
                        _ => Unknown(buf),
                    },
                    AudioSubclass::MidiStream => match MSInterfaceSubtype::from_repr(buf[2]) {
                        Some(MSInterfaceSubtype::MsHeader) => {
                            AudioDescriptorRef::MSInterface(unsafe { &*(buf.as_ptr() as *const _) })
                        }
                        Some(MSInterfaceSubtype::MidiOutJack) => {
                            AudioDescriptorRef::MSOutJack(unsafe { &*(buf.as_ptr() as *const _) })
                        }
                        Some(MSInterfaceSubtype::MidiInJack) => {
                            AudioDescriptorRef::MSInJack(unsafe { &*(buf.as_ptr() as *const _) })
                        }
                        _ => Unknown(buf),
                    },
                },
                DescriptorType::ClassEndpoint => match subclass {
                    AudioSubclass::AudioStream => match ASEndpointSubtype::from_repr(buf[2]) {
                        Some(ASEndpointSubtype::IsochronousEndpoint) => {
                            AudioDescriptorRef::ASEndpoint(unsafe { &*(buf.as_ptr() as *const _) })
                        }
                        _ => Unknown(buf),
                    },
                    AudioSubclass::MidiStream => match MSEndpointSubtype::from_repr(buf[2]) {
                        Some(MSEndpointSubtype::BulkEndpoint) => {
                            AudioDescriptorRef::MSEndpoint(unsafe { &*(buf.as_ptr() as *const _) })
                        }
                        _ => Unknown(buf),
                    },
                    _ => Unknown(buf),
                },
                _ => Unknown(buf),
            };
        }
    }
    Unknown(buf)
}

#[derive(Clone, Copy, Debug, PartialEq, defmt::Format, strum_macros::FromRepr)]
#[repr(u8)]
pub enum ACInterfaceSubtype {
    InterfaceHeader = 0x01,
    InputTerminalDescriptor = 0x02,
    OutputTerminalDescriptor = 0x03,
    FeatureUnitDescriptor = 0x06,
    ClockSourceDescriptor = 0x0A,
    ClockSelectorDescriptor = 0x0B,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ACInterfaceHeaderDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ACInterfaceSubtype,
    pub bcd_adc: u16,
    pub b_category: u8,
    pub w_total_length: u16,
    pub bm_controls: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ACClockSourceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ACInterfaceSubtype,
    pub b_clock_id: u8,
    pub bm_attributes: u8,
    pub bm_controls: u8,
    pub b_assoc_terminal: u8,
    pub i_clock_source: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ACClockSelectorDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ACInterfaceSubtype,
    pub b_clock_id: u8,
    pub b_nr_in_pins: u8,
    pub ba_c_source_id: u8,
    pub bm_controls: u8,
    pub i_clock_selector: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ACFeatureUnitDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ACInterfaceSubtype,
    pub b_unit_id: u8,
    pub b_source_id: u8,
    pub bma_controls_0: u32,
    pub bma_controls_1: u32,
    pub bma_controls_2: u32,
    pub bma_controls_3: u32,
    pub bma_controls_4: u32,
    pub i_feature: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ACInputTerminalDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ACInterfaceSubtype,
    pub b_terminal_id: u8,
    pub w_terminal_type: u16,
    pub b_assoc_terminal: u8,
    pub b_c_source_id: u8,
    pub b_nr_channels: u8,
    pub bm_channel_config: u8,
    pub i_channel_names: u8,
    pub bm_controls: u8,
    pub i_terminal: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ACOutputTerminalDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ACInterfaceSubtype,
    pub b_terminal_id: u8,
    pub w_terminal_type: u16,
    pub b_assoc_terminal: u8,
    pub b_source_id: u8,
    pub b_c_source_id: u8,
    pub bm_controls: u8,
    pub i_terminal: u8,
}

// Audio Stream

#[derive(Clone, Copy, Debug, PartialEq, defmt::Format, strum_macros::FromRepr)]
#[repr(u8)]
pub enum ASInterfaceSubtype {
    AudioStreamHeader = 0x01,
    FormatType1 = 0x02,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ASInterfaceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ASInterfaceSubtype,
    pub b_terminal_link: u8,
    pub bm_controls: u8,
    pub b_format_type: u8,
    pub bm_formats: u32,
    pub b_nr_channels: u8,
    pub bm_channel_config: u32,
    pub i_channel_names: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ASFormatType1Descriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ASInterfaceSubtype,
    pub b_format_type: u8,
    pub b_subslot_size: u8,
    pub b_bit_resolution: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, defmt::Format, strum_macros::FromRepr)]
#[repr(u8)]
pub enum ASEndpointSubtype {
    IsochronousEndpoint = 0x01,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct ASEndpointDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: ASEndpointSubtype,
    pub bm_attributes: u8,
    pub bm_controls: u8,
    pub b_lock_delay_units: u8,
    pub w_lock_delay: u16,
}

// MIDI Stream

#[derive(Clone, Copy, Debug, PartialEq, defmt::Format, strum_macros::FromRepr)]
#[repr(u8)]
pub enum MSInterfaceSubtype {
    MsHeader = 0x01,
    MidiInJack = 0x02,
    MidiOutJack = 0x03,
    Element = 0x04,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct MSInterfaceDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: MSInterfaceSubtype,
    pub bcd_msc: u16,
    pub w_total_length: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct MSInJackDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: MSInterfaceSubtype,
    pub b_jack_type: u8,
    pub b_jack_id: u8,
    pub i_jack: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct MSOutJackDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: MSInterfaceSubtype,
    pub b_jack_type: u8,
    pub b_jack_id: u8,
    pub b_nr_input_pins: u8,
    pub ba_source_id: u8,
    pub ba_source_pin: u8,
    pub i_jack: u8,
}

pub enum JackType {
    Embedded = 1,
    External = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, defmt::Format, strum_macros::FromRepr)]
#[repr(u8)]
pub enum MSEndpointSubtype {
    BulkEndpoint = 0x01,
}

#[derive(Copy, Clone, Debug, PartialEq, defmt::Format)]
#[repr(C)]
pub struct MSEndpointDescriptor {
    pub b_length: u8,
    pub b_descriptor_type: DescriptorType,
    pub b_descriptor_subtype: MSEndpointSubtype,
    pub b_num_emb_midi_jack: u8,
    pub ba_assoc_jack_id: u8,
}
