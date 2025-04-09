use bytes::{BufMut, BytesMut, Buf};

#[derive(Debug, PartialEq)]
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
    pub fn to_bytes(&self) -> BytesMut {
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
                | self.response_code
        );
        buf.put_u16(self.question_count);
        buf.put_u16(self.answer_record_count);
        buf.put_u16(self.authority_record_count);
        buf.put_u16(self.additional_record_count);
        buf
    }

    pub fn from_bytes(bs: &mut BytesMut) -> Header {
        let (id, flags1, flags2) = (bs.get_u16(), bs.get_u8(), bs.get_u8());
        Header {
            packet_identifier: id,
            query_response_indicator: (flags1 & 0b10000000) != 0,
            operation_code: (flags1 & 0b01111000) >> 3,
            authoritative_answer: (flags1 & 0b00000100) != 0,
            truncation: (flags1 & 0b00000010) != 0,
            recursion_desired: (flags1 & 0b00000001) != 0,
            recursion_available: (flags2 & 0b10000000) != 0,
            reserved: (flags2 & 0b01110000) >> 4,
            response_code: flags2 & 0b00001111,
            question_count: bs.get_u16(),
            answer_record_count: bs.get_u16(),
            authority_record_count: bs.get_u16(),
            additional_record_count: bs.get_u16(),
        }
    }
    
}

#[derive(Debug, PartialEq)]
pub struct Question {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
}

impl Question {
    pub fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        put_label_sequence(&mut buf, self.name.as_str());
        buf.put_u16(self.qtype);
        buf.put_u16(self.qclass);
        buf
    }

    pub fn from_bytes(mut bs: &mut BytesMut) -> Question {
        let s = parse_label_sequence(&mut bs);
        Question {
            name: s.join("."),
            qtype: bs.get_u16(),
            qclass: bs.get_u16(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Answer {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
    pub ttl: u32,
    pub length: u16,
    pub data: Vec<u8>
}

impl Answer {
    pub fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        put_label_sequence(&mut buf, self.name.as_str());
        buf.reserve(16 + 16 + 32 + 16); // qtype, qclass, ttl, length
        buf.put_u16(self.qtype);
        buf.put_u16(self.qclass);
        buf.put_u32(self.ttl);
        buf.put_u16(self.length);
        buf.extend(self.data.iter());
        buf
    }

    pub fn from_bytes(mut buf: &mut BytesMut) -> Answer {
        let s = parse_label_sequence(&mut buf);
        Answer {
            name: s.join("."),
            qtype: buf.get_u16(),
            qclass: buf.get_u16(),
            ttl: buf.get_u32(),
            length: buf.get_u16(),
            data: buf.chunk().to_vec()
        }
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

pub fn parse_label_sequence(buf: &mut BytesMut) -> Vec<String> {
    let mut ls: Vec<String> = Vec::new();
    loop { // until we hit that NUL byte \0
        let next = buf.get_u8();
        if next == 0 {
            break;
        }
        let mut consume_bs = next.clone();
        let mut l = Vec::with_capacity(consume_bs as usize);
        while consume_bs != 0 {
            l.push(buf.get_u8());
            consume_bs -= 1;
        }
        ls.push(String::from_utf8(l).unwrap().to_string());
    }
    ls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_to_bytes_from_bytes() {
        let expected = Header {
            packet_identifier: 1234,
            query_response_indicator: true,
            operation_code: 1,
            authoritative_answer: true,
            truncation: false,
            recursion_desired: true,
            recursion_available: false,
            reserved: 0,
            response_code: 3,
            question_count: 1,
            answer_record_count: 2,
            authority_record_count: 0,
            additional_record_count: 0,
        };
        let actual = Header::from_bytes(&mut expected.to_bytes());
        assert_eq!(expected, actual);
        // assert_eq!(expected.packet_identifier, actual.packet_identifier);
        // assert_eq!(expected.query_response_indicator, actual.query_response_indicator);
    }

    #[test]
    fn test_question_to_bytes_from_bytes() {
        let expected = Question {
            name: "example.com".to_string(),
            qtype: 1,  // A record
            qclass: 1, // IN class
        };
        let actual = Question::from_bytes(&mut expected.to_bytes());
        assert_eq!(expected.name, actual.name);
        assert_eq!(expected.qtype, actual.qtype);
        assert_eq!(expected.qclass, actual.qclass);
    }

    #[test]
    fn test_answer_to_bytes_from_bytes() {
        let expected = Answer {
            name: "example.com".to_string(),
            qtype: 0,
            qclass: 1,
            ttl: 360,
            length: 4,
            data: vec![0x08, 0x08, 0x08, 0x08]
        };
        let actual = Answer::from_bytes(&mut expected.to_bytes());
        assert_eq!(expected.name, actual.name);
        assert_eq!(expected.qtype, actual.qtype);
        assert_eq!(expected.qclass, actual.qclass);
        assert_eq!(expected.ttl, actual.ttl);
        assert_eq!(expected.length, actual.length);
        assert_eq!(expected.data, actual.data);
    }
}
