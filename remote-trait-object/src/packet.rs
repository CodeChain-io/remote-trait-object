// Copyright 2020 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::service::MethodId;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct SlotId(u32);

impl fmt::Display for SlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SlotId {{ id: {:?}, type: {:?} }}", self.0, self.get_type())
    }
}

#[derive(Debug)]
pub enum SlotType {
    Request,
    Response,
}

impl SlotId {
    pub fn empty() -> Self {
        Self(999)
    }

    pub fn new(num: u32) -> Self {
        Self(num)
    }

    // FIXME: Please use unique slot id
    pub fn new_request() -> Self {
        Self(1001)
    }

    pub fn get_type(&self) -> SlotType {
        if self.0 >= SLOT_CALL_OR_RETURN_INDICATOR.0 {
            SlotType::Request
        } else {
            SlotType::Response
        }
    }

    pub fn change_to_response(&mut self) {
        assert!(self.0 >= SLOT_CALL_OR_RETURN_INDICATOR.0);
        self.0 -= SLOT_CALL_OR_RETURN_INDICATOR.0;
    }

    pub fn into_request(self) -> Self {
        assert!(self.0 < SLOT_CALL_OR_RETURN_INDICATOR.0);
        Self(self.0 + SLOT_CALL_OR_RETURN_INDICATOR.0)
    }

    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    pub fn as_raw(&self) -> u32 {
        self.0
    }
}

const SLOT_CALL_OR_RETURN_INDICATOR: SlotId = SlotId(1000);

// FIXME: repr(C) is not a reliable encoding method.
// We need to fix the endianness of binary data.
#[repr(C)]
struct PacketHeader {
    pub slot: SlotId,
    // FIXME: use integer indices for the service
    pub target_service_name: [u8; 100],
    pub method: MethodId,
}

impl PacketHeader {
    pub fn len() -> usize {
        std::mem::size_of::<PacketHeader>()
    }

    pub fn new(slot: SlotId, target_service_name: String, method: MethodId) -> Self {
        let mut header = PacketHeader {
            slot,
            target_service_name: [0; 100],
            method,
        };
        copy_string_to_array(target_service_name, &mut header.target_service_name);
        header
    }

    pub fn from_buffer(buffer: &[u8]) -> Self {
        unsafe { std::ptr::read(buffer.as_ptr().cast()) }
    }

    pub fn write(&self, buffer: &mut [u8]) {
        unsafe {
            std::ptr::copy_nonoverlapping(self, buffer.as_mut_ptr().cast(), 1);
        }
    }
}

fn copy_string_to_array(text: String, buffer: &mut [u8]) {
    let text_buffer = text.as_bytes();
    buffer[..text_buffer.len()].copy_from_slice(text_buffer);
}

#[derive(Debug)]
pub struct PacketView<'a> {
    buffer: &'a [u8],
}

impl<'a> fmt::Display for PacketView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Packet {{ slot: {}, service: {}, method: {} }}", self.slot(), self.service_name(), self.method())
    }
}

impl<'a> PacketView<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
        }
    }

    pub fn header(&self) -> &'a [u8] {
        &self.buffer[0..PacketHeader::len()]
    }

    pub fn data(&self) -> &'a [u8] {
        &self.buffer[PacketHeader::len()..]
    }

    pub fn slot(&self) -> SlotId {
        let header = PacketHeader::from_buffer(self.buffer);
        header.slot
    }

    pub fn service_name(&self) -> String {
        let header = PacketHeader::from_buffer(self.buffer);
        let service_name_buffer: Vec<u8> =
            header.target_service_name.iter().cloned().take_while(|char| *char != 0).collect();
        String::from_utf8(service_name_buffer).unwrap()
    }

    pub fn method(&self) -> MethodId {
        let header = PacketHeader::from_buffer(self.buffer);
        header.method
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.buffer.to_vec()
    }

    pub fn to_owned(&self) -> Packet {
        Packet::new_from_buffer(self.buffer.to_vec())
    }
}

#[derive(Debug)]
pub struct Packet {
    buffer: Vec<u8>,
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let view = self.view();
        fmt::Display::fmt(&view, f)
    }
}

impl Packet {
    pub fn new_from_buffer(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
        }
    }

    pub fn new_response_from_request(request: PacketView) -> Self {
        let buffer = request.header().to_vec();
        let mut packet = Self {
            buffer,
        };

        let mut header = packet.header();
        header.slot.change_to_response();
        header.write(&mut packet.buffer);

        packet
    }

    pub fn new_request(service_name: String, method: MethodId, args: &[u8]) -> Self {
        let mut buffer = vec![0 as u8; PacketHeader::len() + args.len()];
        let header = PacketHeader::new(SlotId::new_request(), service_name, method);
        header.write(&mut buffer);
        buffer[PacketHeader::len()..].copy_from_slice(args);
        Self {
            buffer,
        }
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn view(&self) -> PacketView {
        PacketView::new(&self.buffer)
    }

    fn header(&mut self) -> PacketHeader {
        PacketHeader::from_buffer(&self.buffer)
    }

    pub fn data(&self) -> &[u8] {
        self.view().data()
    }

    // FIXME: Use Cursor to reduce data copy
    pub fn append_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn set_slot(&mut self, slot_id: SlotId) {
        let mut header = self.header();
        header.slot = slot_id;
        header.write(&mut self.buffer);
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }
}
