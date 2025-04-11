use bytes::{BufMut, BytesMut, Buf};
use crate::dns::NameResult::{LabelSequence, Pointer, NA};

#[derive(Debug, PartialEq)] // Optional: Derive Debug for easy printing
enum NameResult {
    Pointer(usize),
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

    pub fn from_bytes(buf: &mut BytesMut) -> Record {
        println!("JOHN: Building record");
        let h = Header::from_bytes(buf); // consumes header bytes, e.g. 12
        let bufc = buf.clone(); // Pointer offsets don't include header bytes.
        // Questions
        let mut qs: Vec<Question> = Vec::new();
        for _ in 0..h.question_count {
            println!("JOHN: Question: Resolving questions");
            let mut q = Question::from_bytes(buf);
            if !q.done {
                let Pointer(length_pos) = q.name_result else { panic!("We shouldn't be here.") };
                println!("JOHN: Question: Resolving pointer at byte position {}", length_pos);
                let labels = read_label_sequence(&bufc, length_pos);
                println!("JOHN: Question: Found labels at byte position {}: {:?}", length_pos, labels);
                q.name = labels.join(".");
                q.done = true;
            }
            qs.push(q);
        }
        // Answers
        let mut answers: Vec<Answer> = Vec::new();
        for _ in 0..h.answer_record_count {
            println!("JOHN: Answer: Resolving answers");
            let mut a = Answer::from_bytes(buf);
            if !a.done {
                let Pointer(length_pos) = a.name_result else { panic!("We shouldn't be here.") };
                println!("Answer: Resolving pointer at byte position {}", length_pos);
                let labels = read_label_sequence(&bufc, length_pos);
                println!("Answer: Found labels at byte position {}: {:?}", length_pos, labels);
                a.name = labels.join(".");
                a.done = true;
            }
            answers.push(a);
        }
        Record {
            header: h,
            questions: qs,
            answers: answers,
        }
    }

    pub fn to_bytes(&self) -> BytesMut{
        let (qs, ans): (BytesMut, BytesMut) = (
            self.questions.iter()
                .map(|q| q.to_bytes()).into_iter()
                .fold(BytesMut::new(), |mut acc, bs| { acc.extend_from_slice(&bs); acc}),
            self.answers.iter()
                .map(|a| a.to_bytes()).into_iter()
                .fold(BytesMut::new(), |mut acc, bs| { acc.extend_from_slice(&bs); acc})
        );
        let mut resp = BytesMut::new();
        resp.extend_from_slice(self.header.to_bytes().as_ref());
        resp.extend_from_slice(&qs);
        resp.extend_from_slice(&ans);
        resp
    }
}

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
    done: bool,
}

impl Question {
    fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::new();
        match self.name_result {
            LabelSequence(_) => put_label_sequence(&mut buf, self.name.as_str()),
            Pointer(p) => buf.put_u16((0b11 << 14) | p as u16),
            NA => panic!("what")
        }
        buf.put_u16(self.qtype);
        buf.put_u16(self.qclass);
        buf
    }

    fn from_bytes(buf: &mut BytesMut) -> Question {
        println!("JOHN: Question::from_bytes -- your full bytes: {:?}", buf);
        let nr = parse_name(buf);
        let (name, done) = match &nr {
            LabelSequence(s) => (s.as_str(), true),
            Pointer(_) => ("", false),
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

    pub fn from_question(q: &Question) -> Answer {
        Answer {
            name: q.name.clone(),
            qtype: q.qtype,
            qclass: q.qclass,
            ttl: 60,
            length: 4,
            data: vec![0x08, 0x08, 0x08, 0x07],
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
            Pointer(_) => ("", false),
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
    if is_compressed(buf[0]) {
        println!("JOHN: FOUND COMPRESSION!");
        return Pointer(get_pointer(buf));
    }
    LabelSequence(get_label_sequence(buf).join("."))
}

// TODO: This gets the label sequence, e.g. consumes it. Maybe we don't want to do that?
fn get_label_sequence(buf: &mut BytesMut) -> Vec<String> {
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

fn read_label_sequence(buf: &BytesMut, mut length_pos: usize) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    loop {
        if length_pos == 0x0 {
            break
        }
        let l: usize = buf[length_pos] as usize;
        let (start, end): (usize, usize) = (length_pos + 1, length_pos+1+l);
        labels.push(String::from_utf8(buf[start..end].to_vec()).unwrap());
        length_pos = end + 1
    }
    labels
}

// The pointer is the 2 MSB (big-endian), and we return usize since this will be used for indexing.
fn get_pointer(buf: &mut BytesMut) -> usize {
    (buf.get_u16() << 2 >> 2) as usize // consumes the pointer
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
                answer_record_count: 2,
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
                name: "".to_string(),
                qtype: 0,
                qclass: 1,
                ttl: 360,
                length: 4,
                data: vec![0x08, 0x08, 0x08, 0x08],
                name_result: LabelSequence(String::from("example.com")),
                done: true,
            }],
        };
        println!("{:?}", expected.to_bytes());
        // let actual = Record::from_bytes(&mut expected.to_bytes());
        // assert_eq!(expected, actual);
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
            name_result: Pointer(0b1010),
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
