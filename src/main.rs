#[allow(unused_imports)]
use std::net::UdpSocket;
use bytes::{BytesMut};
use codecrafters_dns_server::dns::{Answer, Record};

fn main() {
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let mut bs = [0; 512];

    loop {
        match udp_socket.recv_from(&mut bs) {
            Ok((size, source)) => {
                let buf = BytesMut::from(&bs[..size]);
                let resp = build_response(buf);
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
    let mut resp = Record::from_bytes(&mut buf);
    resp.header.query_response_indicator = true;
    resp.header.response_code = if resp.header.operation_code == 0 { 0 } else { 4 };
    resp.header.answer_record_count = resp.header.question_count;
    resp.answers = resp.questions.iter().map(|q| { Answer::from_question(q) }).collect();
    println!("{:#?}", resp);
    let out = resp.to_bytes();
    println!("JOHN: raw bytes: {:#?}", out);
    out
}
