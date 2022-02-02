use log::info;
use rand::{prelude::ThreadRng, Rng};
use std::{net::SocketAddr, vec};

mod cache;

use crate::{
    client::Client,
    connection::Connection,
    packets::{Frame, QuestionPacket, ResourceRecordPacket},
};

use self::cache::HashCache;

type ServerResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct Server {
    cache: HashCache,
    rng: ThreadRng,
}

impl Server {
    pub fn new() -> Server {
        Server {
            rng: rand::thread_rng(),
            cache: HashCache::new(),
        }
    }

    pub async fn listen(&mut self, port: usize) -> ServerResult<()> {
        let mut listener = Connection::listen(port).await?;

        loop {
            let (addr, request) = listener.read_frame().await?;

            Server::log_frame(&request, &addr);

            let mut answers: Vec<ResourceRecordPacket> = vec![];

            // Copy of the questions we can mutate
            let mut questions: Vec<QuestionPacket> = request.questions.clone();

            for (i, question) in request.questions.iter().enumerate() {
                // Checking the cache for any answers in the cache
                match self.cache.get(&question.question_type, &question.domain) {
                    Some((record_data, time_to_live)) => {
                        answers.push(ResourceRecordPacket {
                            domain: question.domain.clone(),
                            record_type: question.question_type.clone(),
                            class: crate::packets::ResourceRecordClass::InternetAddress,
                            time_to_live,
                            record_data,
                        });
                        // Remove for questions as we have a cached version
                        questions.remove(i)
                    }
                    None => continue,
                };
            }

            // Check all questions, determine all in the cache
            if request.recursion_desired && !questions.is_empty() {
                let mut recurse_request =
                    Frame::new(self.rng.gen(), crate::packets::PacketType::Query);

                // Add all remaining questions to the recursion packet
                for question in questions.iter() {
                    recurse_request.add_question(question)
                }

                // All the answers given from recursion
                let mut recursion_answers = self.recurse_query(&recurse_request).await?;

                // Add all the answers to the cache
                for answer in recursion_answers.iter() {
                    self.cache.put(
                        &answer.record_type,
                        &answer.domain,
                        &answer.record_data,
                        answer.time_to_live,
                    );
                }

                answers.append(&mut recursion_answers)
            }

            let mut response = request.build_response();

            for question in questions.iter() {
                response.add_question(question);
            }

            for answer in answers.iter() {
                response.add_answer(answer);
            }

            Server::log_frame(&response, &addr);

            listener.write_frame(&response, &addr).await?;
        }
    }

    pub async fn recurse_query(
        &mut self,
        request: &Frame,
    ) -> ServerResult<Vec<ResourceRecordPacket>> {
        let mut client = Client::dial(SocketAddr::from(([8, 8, 8, 8], 53))).await?;

        let response = client.send(request).await?;

        Ok(response.answers)
    }

    pub fn log_frame(frame: &Frame, addr: &SocketAddr) -> Option<Box<dyn std::error::Error>> {
        let mut log = format!("{:?} {} {}", frame.packet_type, addr, frame.id);

        for question in frame.questions.iter() {
            log.push_str(format!(" {:?} {}", question.question_type, question.domain).as_str());
        }

        for answer in frame.answers.iter() {
            log.push_str(
                format!(
                    " {:?} {} {} {:?}",
                    answer.record_type, answer.domain, answer.time_to_live, answer.record_data
                )
                .as_str(),
            );
        }

        info!("{}", log);

        return None;
    }
}
