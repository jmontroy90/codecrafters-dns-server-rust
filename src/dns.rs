use bytes::{Bytes, BufMut, BytesMut};

pub struct Header {
    pub packet_identifier: u16,
    pub query_response_indicator: bool,
    pub operation_code: u8,
    pub authoritative_answer: bool,
    pub truncation: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub reserved: u8,
    pub response_code: u8,
    pub question_count: u16,
    pub answer_record_count: u16,
    pub authority_record_count: u16,
    pub additional_record_count: u16,
}


impl Header {
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(96);
        buf.put_u16(self.packet_identifier);
        buf.put_u8(
            (self.query_response_indicator as u8) << 7
                | (self.operation_code) << 3
                | (self.authoritative_answer as u8) << 2
                | (self.truncation as u8) << 1
                | (self.recursion_desired as u8),
        );
        buf.put_u8(
            (self.recursion_available as u8) << 7
                | (self.reserved) << 4
                | (self.response_code) << 2
        );
        buf.put_u16(self.question_count);
        buf.put_u16(self.answer_record_count);
        buf.put_u16(self.authority_record_count);
        buf.put_u16(self.additional_record_count);
        buf.freeze()
    }
}

pub struct Question {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
}

impl Question {
    pub fn to_bytes(&self) -> Bytes {
        let parts = self.name.split('.');
        // each part is N ASCII characters + 1 size; then 32 bits for qtype and qclass, then the NUL byte
        let cap = (parts.clone().count() + 1) * 8 + 32 + 1;
        let mut buf = BytesMut::with_capacity(cap);
        parts.for_each(|part| {
            buf.put_u8(part.len() as u8);
            for c in part.bytes() {
                if c.is_ascii() {
                    buf.put_u8(c)
                }
            }
        });
        buf.put_u8(0x0); // NUL byte
        buf.put_u16(self.qtype);
        buf.put_u16(self.qclass);
        buf.freeze()
    }
}