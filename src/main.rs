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
                let mut buf = BytesMut::from(&bs[..size]);
                println!("JOHN: Pre trim: {:?}", buf);
                trim_excess_nulls(&mut buf);
                println!("JOHN: Post trim: {:?}", buf);
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

// This is AI, sorryyyyy
fn trim_excess_nulls(buf: &mut BytesMut) {
    let mut last_non_null = buf.len();
    while last_non_null > 0 && buf[last_non_null - 1] == 0 {
        last_non_null -= 1;
    }
    let new_len = (last_non_null + 1).min(buf.len());
    buf.truncate(new_len);
}

fn build_response(mut buf: BytesMut) -> BytesMut {
    let mut resp = Record::from_bytes(&mut buf);
    resp.header.query_response_indicator = true;
    resp.header.response_code = if resp.header.operation_code == 0 { 0 } else { 4 };
    resp.header.answer_record_count = resp.header.question_count;
    resp.answers = resp.questions.iter().map(|q| { Answer::from_question(q) }).collect();
    resp.to_bytes()
}
