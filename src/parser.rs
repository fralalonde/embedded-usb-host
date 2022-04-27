use utf16string::{LE, WStr};

use crate::{Class, DeviceDescriptor, InterfaceAssociationDescriptor};
use crate::class::audio;
use crate::class::audio::AudioDescriptorRef;
use crate::descriptor::{ConfigurationDescriptor, DescriptorType, EndpointDescriptor, InterfaceDescriptor};

#[derive(Debug, defmt::Format)]
pub enum DescriptorRef<'a> {
    Device(&'a DeviceDescriptor),
    Configuration(&'a ConfigurationDescriptor),
    String(&'a WStr<LE>),
    Interface(&'a InterfaceDescriptor),
    Endpoint(&'a EndpointDescriptor),

    InterfaceAssociation(&'a InterfaceAssociationDescriptor),

    Audio(AudioDescriptorRef<'a>),

    UnknownClassInterface(&'a [u8]),
    UnknownClassEndpoint(&'a [u8]),

    Unknown(&'a [u8]),
}

pub struct DescriptorParser<'a> {
    buf: &'a [u8],
    pos: usize,
    class: Option<Class>,
    subclass: Option<u8>,
}

impl<'a> Iterator for DescriptorParser<'a> {
    type Item = DescriptorRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buf.len() {
            // we're done here
            return None;
        }

        let desc_len = self.buf[self.pos] as usize;
        if desc_len == 0 {
            warn!("Invalid descriptor of len 0");
            return None;
        }

        if self.pos + desc_len > self.buf.len() {
            warn!("truncated descriptor of len {}", desc_len);
            return None;
        }

        let desc_next = self.pos + desc_len;

        let desc_type: u8 = self.buf[self.pos + 1];
        let desc_offset = unsafe { self.buf.as_ptr().add(self.pos as usize) };
        let body_offset = unsafe { self.buf.as_ptr().add((self.pos + 2) as usize) };

        let desc_ref = match DescriptorType::from_repr(desc_type) {
            Some(DescriptorType::Device) => Some(DescriptorRef::Device(unsafe { &*(desc_offset as *const _) })),
            Some(DescriptorType::Configuration) => Some(DescriptorRef::Configuration(unsafe { &*(desc_offset as *const _) })),
            Some(DescriptorType::String) => Some(DescriptorRef::String(unsafe { WStr::from_utf16le_unchecked(core::slice::from_raw_parts(body_offset as *const _, (desc_len - 2) as usize)) })),
            Some(DescriptorType::Interface) => {
                let ifdesc: &InterfaceDescriptor = unsafe { &*(desc_offset as *const _) };
                if ifdesc.b_interface_class != 0 && ifdesc.b_interface_sub_class != 0 {
                    self.class = Class::from_repr(ifdesc.b_interface_class);
                    self.subclass = Some(ifdesc.b_interface_sub_class);
                }
                Some(DescriptorRef::Interface(ifdesc))
            }
            Some(DescriptorType::Endpoint) => Some(DescriptorRef::Endpoint(unsafe { &*(desc_offset as *const _) })),
            Some(DescriptorType::InterfaceAssociation) => Some(DescriptorRef::InterfaceAssociation(unsafe { &*(desc_offset as *const _) })),

            Some(DescriptorType::ClassInterface) if self.class == Some(Class::Audio) => Some(DescriptorRef::Audio(audio::parse(self.subclass, DescriptorType::ClassInterface, &self.buf[self.pos..desc_next]))),
            Some(DescriptorType::ClassEndpoint) if self.class == Some(Class::Audio) => Some(DescriptorRef::Audio(audio::parse(self.subclass, DescriptorType::ClassEndpoint, &self.buf[self.pos..desc_next]))),

            Some(DescriptorType::ClassInterface) => Some(DescriptorRef::UnknownClassInterface(&self.buf[self.pos..desc_next])),
            Some(DescriptorType::ClassEndpoint) => Some(DescriptorRef::UnknownClassEndpoint(&self.buf[self.pos..desc_next])),

            _ => Some(DescriptorRef::Unknown(&self.buf[self.pos..desc_next])),
        };

        // advance to next descriptor
        self.pos = desc_next;
        return desc_ref;
    }
}

impl<'a> DescriptorParser<'a> {
    // TODO earlier DeviceDesc might provide class and subclass instead of interfaces
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0, class: None, subclass: None }
    }

    pub fn rewind(&mut self) {
        self.pos = 0;
    }
}