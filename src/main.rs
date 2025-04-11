#[allow(unused_imports)]
use std::net::UdpSocket;
use bytes::{BytesMut};
use codecrafters_dns_server::dns::{Question, Answer, Record};
use std::env;

fn main() {

    let args: Vec<String> = env::args().collect();
    let argv: Vec<&str> = args.iter().map(|x| &x[..]).collect();
    match argv.as_slice() {
        [_, "--resolver", addr] => resolve_with_upstream(addr), // assuming this is the only mode now?
        _ => eprintln!("Oh no!!")
    }
}

fn resolve_with_upstream(upstream_addr: &str) {

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to local");
    let upstream_socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to upstream");
    let mut bs = [0; 512];

    println!("JOHN: starting main loop");
    loop {
        match udp_socket.recv_from(&mut bs) {
            Ok((size, source)) => {
                println!("Received {} bytes from client {}", size, source);
                let req = Record::from_bytes(&mut BytesMut::from(&bs[..size]));
                let resps: Vec<Record> = req
                    .generate_single_requests()
                    .into_iter()
                    .map(|req| { resolve_req(&upstream_socket, upstream_addr, req) }).collect();

                let mut h = resps[0].header.clone();
                // Consumes the resps...
                let (qs, ans): (Vec<Question>, Vec<Answer>) = resps
                    .into_iter()
                    .fold((Vec::new(), Vec::new()), |(mut qs, mut ans), r| {
                        qs.extend(r.questions);
                        ans.extend(r.answers);
                        (qs, ans)
                    });
                h.question_count = qs.len() as u16;
                h.answer_record_count = ans.len() as u16;
                let bs = Record { header: h, questions: qs, answers: ans}.to_bytes();
                udp_socket.send_to(&bs, &source).unwrap();
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}

fn resolve_req(upstream_socket: &UdpSocket, dest: &str, req: Record) -> Record {
    let mut buf = [0u8; 512];
    upstream_socket.send_to(&req.to_bytes(), dest).expect("Failed to send to upstream");
    let (amt, src) = upstream_socket.recv_from(&mut buf).expect("Failed to receive from upstream");
    println!("Received {} bytes from upstream: {}", amt, src);
    Record::from_bytes(&mut BytesMut::from(&buf[..amt]))
}

fn build_response(mut buf: BytesMut) -> BytesMut {
    let mut resp = Record::from_bytes(&mut buf);
    resp.header.query_response_indicator = true;
    resp.header.response_code = if resp.header.operation_code == 0 { 0 } else { 4 };
    resp.header.answer_record_count = resp.header.question_count;
    resp.answers = resp.questions.iter().enumerate().map(|(i, q)| { Answer::from_question(i as u8, q) }).collect();
    let out = resp.to_bytes();
    out
}
