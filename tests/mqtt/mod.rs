use std::{
    cmp::Ordering,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::{Duration, Instant},
};

use rumqttc::Publish;

pub struct CollectingClient {
    received_messages: std::sync::Arc<Mutex<Vec<Publish>>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
    config: ha_mitaffald::settings::MQTTConfig,
    terminate_flag: Arc<AtomicBool>,
}

impl CollectingClient {
    pub fn new(config: &ha_mitaffald::settings::MQTTConfig) -> Self {
        let config = ha_mitaffald::settings::MQTTConfig {
            client_id: "collecting-client".to_owned(),
            ..config.clone()
        };

        Self {
            config,
            received_messages: Arc::new(Mutex::new(Vec::new())),
            join_handle: None,
            terminate_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self) {
        let config = self.config.clone();
        let received_messages = self.received_messages.clone();
        let stopping_flag = Arc::clone(&self.terminate_flag);
        let (mut client, mut connection) = rumqttc::Client::new(config.into(), 100);
        client.subscribe("#", rumqttc::QoS::AtLeastOnce).unwrap();

        let handle = std::thread::spawn(move || loop {
            let message = connection.recv_timeout(Duration::from_secs(1));

            println!("Received message: {:?}", message);
            if let Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(message)))) = message {
                received_messages.lock().unwrap().push(message);
            }

            if stopping_flag.load(std::sync::atomic::Ordering::Relaxed) {
                println!("Thread is terminating");
                break;
            }
        });

        self.join_handle = Some(handle);
    }

    pub fn wait_for_messages(
        self,
        count: usize,
        timeout: Duration,
    ) -> Result<Vec<Publish>, WaitError> {
        let start = std::time::Instant::now();
        loop {
            let received_messages = self.received_messages.lock().unwrap().len();
            if received_messages >= count {
                self.terminate_flag
                    .store(true, std::sync::atomic::Ordering::Relaxed);

                break;
            }

            if Instant::now() - start > timeout {
                self.terminate_flag
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        println!("Joining worker thread...");
        if let Some(handle) = self.join_handle {
            handle.join().unwrap();
        }
        //todo: can get rid of clone here?
        let received_messages = self.received_messages.lock().unwrap();

        match received_messages.len().cmp(&count) {
            Ordering::Equal => Ok(received_messages.clone()),
            Ordering::Greater => Err(WaitError::TooMany(received_messages.clone())),
            Ordering::Less => Err(WaitError::Timeout(received_messages.clone())),
        }
    }
}

#[derive(Debug)]
pub enum WaitError {
    Timeout(Vec<Publish>),
    TooMany(Vec<Publish>),
}
