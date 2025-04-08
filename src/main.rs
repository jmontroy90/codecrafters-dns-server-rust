#[allow(unused_imports)]
use std::net::UdpSocket;
use codecrafters_dns_server::dns;
use bytes::{BytesMut};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut buf = [0; 512];

    let header = dns::Header {
        packet_identifier: 1234,
        query_response_indicator: true,
        operation_code: 0,
        authoritative_answer: false,
        truncation: false,
        recursion_desired: false,
        recursion_available: false,
        reserved: 0,
        response_code: 0,
        question_count: 1,
        answer_record_count: 1,
        authority_record_count: 0,
        additional_record_count: 0,
    };

    let question = dns::Question {
        name: String::from("codecrafters.io"),
        qtype: 1,
        qclass: 1,
    };

    let answer = dns::Answer {
        name: String::from("codecrafters.io"),
        qtype: 1,
        qclass: 1,
        ttl: 60,
        length: 4,
        data: b"\x08\x08\x08\x08".to_owned()
    };

    let cap = header.to_bytes().len() + question.to_bytes().len() + answer.to_bytes().len();
    let mut resp = BytesMut::with_capacity(cap);
    resp.extend_from_slice(&header.to_bytes());
    resp.extend_from_slice(&question.to_bytes());
    resp.extend_from_slice(&answer.to_bytes());
    let out = resp.freeze();

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);
                udp_socket
                    .send_to(&out, source)
                    .expect("Failed to send response");
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
