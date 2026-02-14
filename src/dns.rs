use std::net::SocketAddr;

use anyhow::{Context, Result};
use simple_dns::rdata::RData;
use simple_dns::{CLASS, Packet, QTYPE, Question, ResourceRecord, TYPE};
use tokio::net::UdpSocket;
use tracing::{debug, error, info};

const TTL: u32 = 60;
const LOCALHOST_IP: [u8; 4] = [127, 0, 0, 1];

pub async fn run_dns_server(addr: SocketAddr) -> Result<()> {
    let socket = UdpSocket::bind(addr)
        .await
        .with_context(|| format!("failed to bind DNS server to {addr}"))?;

    info!(%addr, "DNS server listening");

    let mut buf = vec![0u8; 512];

    loop {
        let (len, src) = match socket.recv_from(&mut buf).await {
            Ok(result) => result,
            Err(e) => {
                error!(error = %e, "failed to receive DNS packet");
                continue;
            }
        };

        let response = match handle_query(&buf[..len]) {
            Ok(resp) => resp,
            Err(e) => {
                debug!(error = %e, "failed to handle DNS query");
                continue;
            }
        };

        if let Err(e) = socket.send_to(&response, src).await {
            error!(error = %e, %src, "failed to send DNS response");
        }
    }
}

fn handle_query(data: &[u8]) -> Result<Vec<u8>> {
    let query = Packet::parse(data).context("failed to parse DNS query")?;

    let mut response = Packet::new_reply(query.id());

    for question in &query.questions {
        let name_str = question.qname.to_string();

        if is_test_domain(&name_str) && question.qtype == QTYPE::TYPE(TYPE::A) {
            debug!(name = %name_str, "resolving .test domain");
            response.questions.push(Question::new(
                question.qname.clone(),
                question.qtype,
                question.qclass,
                question.unicast_response,
            ));
            let rdata = RData::A(simple_dns::rdata::A {
                address: u32::from_be_bytes(LOCALHOST_IP),
            });
            let record = ResourceRecord::new(question.qname.clone(), CLASS::IN, TTL, rdata);
            response.answers.push(record);
        }
    }

    let bytes = response
        .build_bytes_vec()
        .context("failed to build DNS response")?;
    Ok(bytes)
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_test_domain(name: &str) -> bool {
    let name = name.trim_end_matches('.');
    name.eq_ignore_ascii_case("test") || name.to_ascii_lowercase().ends_with(".test")
}

#[cfg(test)]
mod tests {
    use simple_dns::{Name, QCLASS};

    use super::*;

    fn build_query(name: &str) -> Vec<u8> {
        let mut packet = Packet::new_query(1234);
        packet.questions.push(Question::new(
            Name::new_unchecked(name),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.build_bytes_vec().unwrap()
    }

    #[test]
    fn resolves_test_domain() {
        let query = build_query("hello.test");
        let response_bytes = handle_query(&query).unwrap();
        let response = Packet::parse(&response_bytes).unwrap();

        assert_eq!(response.answers.len(), 1);
        match &response.answers[0].rdata {
            RData::A(a) => {
                assert_eq!(a.address, u32::from_be_bytes(LOCALHOST_IP));
            }
            other => panic!("expected A record, got {other:?}"),
        }
    }

    #[test]
    fn ignores_non_test_domain() {
        let query = build_query("hello.com");
        let response_bytes = handle_query(&query).unwrap();
        let response = Packet::parse(&response_bytes).unwrap();

        assert_eq!(response.answers.len(), 0);
    }
}
