use std::collections::HashMap;
use std::{net::SocketAddr, sync::Arc};

use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use tokio::{net::UdpSocket, sync::RwLock};

use tokio::sync::mpsc;

use crate::messages::errors::ClientError;
use crate::messages::packets::{Question, QuestionClass, ResponseCode};

use super::{
    coding::MessageCoder,
    network_buffer::NetworkBuffer,
    packets::{Message, PacketType, ResourceRecordType},
};

type ClientResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Client {
    addr: SocketAddr,
    sock: Arc<UdpSocket>,
    rng: RwLock<StdRng>,
}

impl Client {
    /// Dial and connect to a remote address. The client will only read messages from the given remote address.
    pub async fn dial(addr: SocketAddr) -> ClientResult<Client> {
        // Bind our socket
        let sock = Arc::new(UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?);

        // Connect socket to address, so we only receive messages from that address
        sock.connect(addr).await?;

        let rng: RwLock<StdRng> = RwLock::new(SeedableRng::from_entropy());

        Ok(Client { addr, sock, rng })
    }

    /// Send request to connected upstream server
    pub async fn send(&self, message: &Message, buf: &mut NetworkBuffer) -> ClientResult<()> {
        // Encode the message, MessageCoder instances should be ephemeral
        MessageCoder::new().encode_message(message, buf)?;

        // Only write the length of the buffer
        let buffer_length = buf.write_count();

        let write_count = self
            .sock
            .send_to(&buf.buf[..buffer_length], self.addr)
            .await?;

        // Reset the buffer
        buf.reset();

        Ok(())
    }

    async fn generate_id(&self) -> u16 {
        self.rng.write().await.gen()
    }

    pub async fn query(
        &self,
        domain: &str,
        request_type: ResourceRecordType,
    ) -> ClientResult<Message> {
        let mut buf = NetworkBuffer::new();

        // Create RNG to generate ID's for queries

        let message = Message {
            id: self.generate_id().await,
            packet_type: PacketType::Query,
            op_code: 0,
            authoritative_answer: false,
            truncation: false,
            recursion_desired: true,
            recursion_available: false,
            response_code: ResponseCode::None,
            // Single question
            questions: vec![Question {
                domain: domain.to_string(),
                question_type: request_type,
                class: QuestionClass::InternetAddress,
            }],
            answers: vec![],
            authorities: vec![],
            additional_records: vec![],
        };

        // Send the message
        self.send(&message, &mut buf).await?;

        // Read datagram from socket
        let (_len, addr) = self.sock.recv_from(&mut buf.buf).await.unwrap();

        // Decode message
        let message = MessageCoder::new().decode_message(&mut buf).unwrap();

        return Ok(message);
    }
}
