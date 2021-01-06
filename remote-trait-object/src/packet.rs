use crate::forwarder::ServiceObjectId;
use crate::service::MethodId;
use serde::{Deserialize, Serialize};
use std::fmt;

const UNDECIDED_SLOT: u32 = 4_294_967_295;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
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
    pub fn new(num: u32) -> Self {
        Self(num)
    }

    pub fn new_request() -> Self {
        Self(UNDECIDED_SLOT)
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

const SLOT_CALL_OR_RETURN_INDICATOR: SlotId = SlotId(2_147_483_648);

/// FIXME: Replace this hard-coded value to some constant evaluation
const PACKET_HEADER_SIZE: usize = 12;

#[test]
fn packet_header_size() {
    let x = PacketHeader {
        slot: SlotId(0),
        service_object_id: 0,
        method: 0,
    };
    assert_eq!(bincode::serialize(&x).unwrap().len(), PACKET_HEADER_SIZE);
}

#[derive(Serialize, Deserialize)]
struct PacketHeader {
    pub slot: SlotId,
    pub service_object_id: ServiceObjectId,
    pub method: MethodId,
}

impl PacketHeader {
    pub const fn len() -> usize {
        PACKET_HEADER_SIZE
    }

    pub fn new(slot: SlotId, service_object_id: ServiceObjectId, method: MethodId) -> Self {
        PacketHeader {
            slot,
            service_object_id,
            method,
        }
    }

    pub fn from_buffer(buffer: &[u8]) -> Self {
        bincode::deserialize_from(buffer).unwrap()
    }

    pub fn write(&self, buffer: &mut [u8]) {
        bincode::serialize_into(buffer, self).unwrap()
    }
}

#[derive(Debug)]
pub struct PacketView<'a> {
    buffer: &'a [u8],
}

impl<'a> fmt::Display for PacketView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Packet {{ slot: {}, object id: {}, method: {} }}", self.slot(), self.object_id(), self.method())
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

    pub fn object_id(&self) -> ServiceObjectId {
        PacketHeader::from_buffer(self.buffer).service_object_id
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

    pub fn new_request(service_object_id: ServiceObjectId, method: MethodId, args: &[u8]) -> Self {
        let mut buffer = vec![0_u8; PacketHeader::len() + args.len()];
        let header = PacketHeader::new(SlotId::new_request(), service_object_id, method);
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
