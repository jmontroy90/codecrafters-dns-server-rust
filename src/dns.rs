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
        let mut buf = BytesMut::new();
        put_label_sequence(&mut buf, self.name.as_str());
        buf.put_u16(self.qtype);
        buf.put_u16(self.qclass);
        buf.freeze()
    }
}

pub struct Answer {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
    pub ttl: u32,
    pub length: u16,
    pub data: [u8; 4]
}

impl Answer {
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::new();
        put_label_sequence(&mut buf, self.name.as_str());
        buf.reserve(16 + 16 + 32 + 16 + 32);
        buf.put_u16(self.qtype);
        buf.put_u16(self.qclass);
        buf.put_u32(self.ttl);
        buf.put_u16(self.length);
        buf.extend(self.data);
        buf.freeze()
    }
}

pub fn put_label_sequence(buf: &mut BytesMut, raw: &str) {
    let parts = raw.split('.');
    let cap = label_sequence_cap(parts.clone().count());
    buf.reserve(cap);
    parts.for_each(|part| {
        buf.put_u8(part.len() as u8);
        for c in part.bytes() {
            if c.is_ascii() {
                buf.put_u8(c)
            }
        }
    });
    buf.put_u8(0x0); // NUL byte
}

// each part is N ASCII characters + 1 size; then 32 bits for qtype and qclass, then the NUL byte
pub fn label_sequence_cap(num_labels: usize) -> usize {
    ((num_labels + 1) * 8) + 1
}
