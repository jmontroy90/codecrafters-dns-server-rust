use bytes::{BufMut, BytesMut, Buf};
use crate::dns::NameResult::{LabelSequence, Pointer, NA};

#[derive(Debug, PartialEq)]
enum NameResult {
    Pointer(String, usize),
    LabelSequence(String),
    NA
}

#[derive(Debug, PartialEq)]
pub struct Record {
    pub header: Header,
    pub questions: Vec<Question>,
    pub answers: Vec<Answer>
}

impl Record {

    // TODO: lots of redundant code here, and weird types with String
    pub fn from_bytes(buf: &mut BytesMut) -> Record {
        let bufc = buf.clone(); // Used for resolving pointers.
        let h = Header::from_bytes(buf); // consumes header bytes, e.g. 12
        // Questions
        let mut qs: Vec<Question> = Vec::new();
        for _ in 0..h.question_count {
            let mut q = Question::from_bytes(buf);
            if !q.done {
                q.update_pointer_from_full(&bufc);
            }
            qs.push(q);
        }
        // Answers
        let mut answers: Vec<Answer> = Vec::new();
        for _ in 0..h.answer_record_count {
            let mut a = Answer::from_bytes(buf);
            if !a.done {
                a.update_pointer_from_full(&bufc)
            }
            answers.push(a);
        }
        Record {
            header: h,
            questions: qs,
            answers: answers,
        }
    }

    pub fn to_bytes(&self) -> BytesMut {
        let (qs, ans): (BytesMut, BytesMut) = (
            self.questions.iter()
                .map(|q| q.to_bytes()).into_iter()
                .fold(BytesMut::new(), |mut acc, bs| { acc.extend_from_slice(&bs); acc}),
            self.answers.iter()
                .map(|a| a.to_bytes()).into_iter()
                .fold(BytesMut::new(), |mut acc, bs| { acc.extend_from_slice(&bs); acc})
        );
        [self.header.to_bytes(), qs, ans].iter().fold(BytesMut::new(), |mut acc, bs| { acc.extend_from_slice(&bs); acc })
    }

    // Consumes self?!?!
    pub fn generate_single_requests(self) -> Vec<Record> {
        let mut h = self.header.clone();
        h.question_count = 1;
        self.questions.into_iter().map(|q| {
            Record { header: h.clone(), questions: vec![q], answers: vec![]}
        }).collect::<Vec<Record>>()
    }
}

#[derive(Debug, PartialEq, Clone)]
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

    pub fn from_bytes(buf: &mut BytesMut) -> Header {
        let (id, flags1, flags2) = (buf.get_u16(), buf.get_u8(), buf.get_u8());
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
            question_count: buf.get_u16(),
            answer_record_count: buf.get_u16(),
            authority_record_count: buf.get_u16(),
            additional_record_count: buf.get_u16(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Question {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,

    name_result: NameResult,
    // TODO: This flag is redundant with the NameResult type; if it's Pointer, I think we can always say it's not done.
    done: bool,
}

impl Question {
    fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        match &self.name_result {
            LabelSequence(_) => put_label_sequence(&mut buf, self.name.as_str()),
            Pointer(existing, _) => put_label_sequence(&mut buf, existing.as_str()), // buf.put_u16((0b11 << 14) | *p as u16)
            NA => panic!("what")
        }
        buf.put_u16(self.qtype);
        buf.put_u16(self.qclass);
        buf
    }

    fn from_bytes(buf: &mut BytesMut) -> Question {
        let nr = parse_name(buf);
        let (name, done) = match &nr {
            LabelSequence(s) => (s.as_str(), true),
            Pointer(existing, _) => (existing.as_str(), false),
            NA => panic!("what")
        };
        Question {
            name: name.to_string(),
            name_result: nr,
            done: done,
            qtype: buf.get_u16(),
            qclass: buf.get_u16(),
        }
    }

    fn update_pointer_from_full(&mut self, buf: &BytesMut) {
        let Pointer(ref mut existing, start_pos) = self.name_result else { panic!("We shouldn't be here.") };
        let labels = read_label_sequence(buf, start_pos);
        existing.push('.');
        existing.push_str(labels.join(".").as_str());
        self.name = existing.to_string();
        self.done = true;
    }
}

#[derive(Debug, PartialEq)]
pub struct Answer {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
    pub ttl: u32,
    pub length: u16,
    pub data: Vec<u8>,

    name_result: NameResult,
    done: bool
}

impl Answer {

    pub fn from_question(i: u8, q: &Question) -> Answer {
        Answer {
            name: q.name.clone(),
            qtype: q.qtype,
            qclass: q.qclass,
            ttl: 60,
            length: 4,
            data: vec![0x8, 0x8, 0x8, 0x8 + i],
            name_result: NA,
            done: true
        }
    }
    pub fn to_bytes(&self) -> BytesMut {
        if !self.done {
            panic!("Answer::to_bytes contains unresolved pointers");
        }
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

    pub fn from_bytes(buf: &mut BytesMut) -> Answer {
        let nr = parse_name(buf);
        let (name, done) = match &nr {
            LabelSequence(s) => (s.as_str(), true),
            Pointer(existing, _) => (existing.as_str(), false),
            NA => panic!("what")
        };
        Answer {
            name: name.to_string(),
            qtype: buf.get_u16(),
            qclass: buf.get_u16(),
            ttl: buf.get_u32(),
            length: buf.get_u16(),
            data: buf.chunk().to_vec(),
            name_result: nr,
            done: done
        }
    }

    fn update_pointer_from_full(&mut self, buf: &BytesMut) {
        let Pointer(ref mut existing, length_pos) = self.name_result else { panic!("We shouldn't be here.") };
        let labels = read_label_sequence(&buf, length_pos);
        existing.push('.');
        existing.push_str(labels.join(".").as_str());
        self.name = existing.to_string();
        self.done = true;
    }
}

fn put_label_sequence(buf: &mut BytesMut, raw: &str) {
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
fn label_sequence_cap(num_labels: usize) -> usize {
    ((num_labels + 1) * 8) + 1
}

// Does not consume the buf!
fn is_compressed(buf: u8) -> bool {
    buf >> 6 == 0b11
}

fn parse_name(buf: &mut BytesMut) -> NameResult  {
    // This consumes the buffer, and handles the THREE cases of the QNAME field.
    let mut ls: Vec<String> = Vec::new();
    loop {
        let next = buf.get_u8();
        if is_compressed(next) {
            let pbs = [next << 2 >> 2, buf.get_u8()];
            let (existing, pointer) = (ls.join("."), u16::from_be_bytes(pbs) as usize);
            return Pointer(existing, pointer);
        } else if next == 0x0 { // Pointers will never end with \0
            return LabelSequence(ls.join("."));
        } else {
            let mut consume_bs = next.clone();
            let mut l = Vec::with_capacity(consume_bs as usize);
            while consume_bs != 0 {
                l.push(buf.get_u8());
                consume_bs -= 1;
            }
            ls.push(String::from_utf8(l).unwrap().to_string());
        }
    }
}

fn read_label_sequence(buf: &BytesMut, mut start_pos: usize) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    loop {
        let length = buf[start_pos] as usize;
        if length == 0x0 {
            break
        }
        let (start, end): (usize, usize) = (start_pos + 1, start_pos+1+length);
        labels.push(String::from_utf8(buf[start..end].to_vec()).unwrap());
        start_pos = end
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_to_bytes_from_bytes() {
        let expected = Record {
            header: Header {
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
                answer_record_count: 1,
                authority_record_count: 0,
                additional_record_count: 0,
            },
            questions: vec![Question {
                name: "example.com".to_string(),
                qtype: 1,  // A record
                qclass: 1, // IN class
                name_result: LabelSequence("example.com".to_string()),
                done: true,
            }],
            answers: vec![Answer {
                name: "example.com".to_string(),
                qtype: 1,
                qclass: 1,
                ttl: 360,
                length: 4,
                data: vec![0x08, 0x08, 0x08, 0x08],
                name_result: LabelSequence(String::from("example.com")),
                done: true,
            }],
        };
        let actual = Record::from_bytes(&mut expected.to_bytes());
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_record_from_bytes() {
        let mut bs = BytesMut::new();
        bs.extend_from_slice(b"\x11\xb3\x81\0\0\x01\0\x01\0\0\0\0\x0ccodecrafters\x02io\0\0\x01\0\x01\x0ccodecrafters\x02io\0\0\x01\0\x01\0\0\0<\0\x04\x08\x08\x08\x08".as_slice());
        let record = Record::from_bytes(&mut bs);
        println!("{:?}", record);
    }

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
    }

    #[test]
    fn test_question_to_bytes_from_bytes() {
        let expected = Question {
            name: "example.com".to_string(),
            qtype: 1,  // A record
            qclass: 1, // IN class
            name_result: LabelSequence("example.com".to_string()),
            done: true,
        };
        let actual = Question::from_bytes(&mut expected.to_bytes());
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_question_to_bytes_pointer() {
        let expected = Question {
            name: String::from(""),
            qtype: 1,  // A record
            qclass: 1, // IN class
            name_result: Pointer("".to_string(), 0b1010),
            done: false,
        };
        let bs = expected.to_bytes();
        assert!(is_compressed(bs[0]));
        assert_eq!(&[bs[0], bs[1]], &[0b1100_0000, 0b0000_1010]);
    }

    #[test]
    fn test_answer_to_bytes_from_bytes() {
        let expected = Answer {
            name: "example.com".to_string(),
            qtype: 0,
            qclass: 1,
            ttl: 360,
            length: 4,
            data: vec![0x08, 0x08, 0x08, 0x08],
            name_result: LabelSequence(String::from("example.com")),
            done: true,
        };
        let actual = Answer::from_bytes(&mut expected.to_bytes());
        assert_eq!(expected, actual);
    }
}
