use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::{Duration, Instant},
};

use rumqttc::{Client, Event, Packet, Publish, QoS};
use tracing::info;

pub struct CollectingClient {
    received_messages: std::sync::Arc<Mutex<Vec<Publish>>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
    terminate_flag: Arc<AtomicBool>,
}

impl CollectingClient {
    pub fn new() -> Self {
        Self {
            received_messages: Arc::new(Mutex::new(Vec::new())),
            join_handle: None,
            terminate_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self, config: &ha_mitaffald::settings::MQTTConfig) {
        let config = ha_mitaffald::settings::MQTTConfig {
            client_id: "collecting-client".to_owned(),
            ..config.clone()
        };
        let received_messages = self.received_messages.clone();
        let stopping_flag = Arc::clone(&self.terminate_flag);
        let (tx, rx) = std::sync::mpsc::channel::<()>();

        let handle = std::thread::spawn(move || {
            let (client, mut connection) = Client::new(config.into(), 100);
            client.subscribe("#", QoS::AtLeastOnce).unwrap();

            loop {
                let message = connection.recv_timeout(Duration::from_secs(1));
                info!("Received message: {:?}", &message);
                match message {
                    Ok(Ok(Event::Incoming(Packet::SubAck(_)))) => {
                        tx.send(()).expect("Cannot report ready to main thread")
                    }
                    Ok(Ok(Event::Incoming(Packet::Publish(message)))) => {
                        received_messages.lock().unwrap().push(message);
                    }
                    _ => {}
                }

                if stopping_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    info!("Thread is terminating");
                    break;
                }
            }
        });

        rx.recv().expect("Consumer thread did not report ready");
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

        info!("Joining worker thread...");
        if let Some(handle) = self.join_handle {
            handle.join().unwrap();
        }

        let inner_mutex =
            Arc::try_unwrap(self.received_messages).expect("More than one reference detected");
        let received_messages = inner_mutex.into_inner().unwrap();

        match received_messages.len().cmp(&count) {
            Ordering::Equal => Ok(received_messages),
            Ordering::Greater => Err(WaitError::TooMany(received_messages)),
            Ordering::Less => Err(WaitError::Timeout(received_messages)),
        }
    }
}

#[derive(Debug)]
pub enum WaitError {
    Timeout(Vec<Publish>),
    TooMany(Vec<Publish>),
}

impl Display for WaitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let payload_print = |f: &mut Formatter<'_>, publishes: &Vec<Publish>| -> fmt::Result {
            for publish in publishes {
                writeln!(
                    f,
                    "({} : {}), ",
                    publish.topic,
                    String::from_utf8(publish.payload.to_vec()).unwrap()
                )?;
            }

            Ok(())
        };

        match self {
            WaitError::Timeout(publishes) => {
                write!(f, "Timeout: [")?;
                payload_print(f, publishes)?;
                write!(f, "]")
            }
            WaitError::TooMany(publishes) => {
                write!(f, "TooMany: [")?;
                payload_print(f, publishes)?;
                write!(f, "]")
            }
        }
    }
}
