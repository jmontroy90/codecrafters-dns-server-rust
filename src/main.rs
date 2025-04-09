#[allow(unused_imports)]
use std::net::UdpSocket;
use codecrafters_dns_server::dns;
use bytes::{BytesMut};

fn main() {
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                let resp = build_response(bytes::BytesMut::from(&buf[..]));
                println!("Received {} bytes from {}", size, source);
                udp_socket
                    .send_to(&resp.freeze(), source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}

fn build_response(mut buf: BytesMut) -> BytesMut {
    let mut resp_header = dns::Header::from_bytes(&mut buf);
    resp_header.query_response_indicator = true;
    resp_header.response_code = if resp_header.operation_code == 0 { 0 } else { 4 };
    resp_header.answer_record_count = 1;
    let resp_question = dns::Question::from_bytes(&mut buf);
    let mut resp_answer = dns::Answer::from_bytes(&mut buf);
    resp_answer.name = resp_question.name.clone();
    resp_answer.qclass = resp_question.qclass;
    resp_answer.qtype = resp_question.qtype;

    let (resp_header_bs, resp_question_bs, resp_answer_bs) = (resp_header.to_bytes(), resp_question.to_bytes(), resp_answer.to_bytes());
    let cap = resp_header_bs.len() + resp_question_bs.len() + resp_answer_bs.len();
    let mut resp = BytesMut::with_capacity(cap);
    resp.extend_from_slice(&resp_header_bs);
    resp.extend_from_slice(&resp_question_bs);
    resp.extend_from_slice(&resp_answer_bs);
    resp
}
