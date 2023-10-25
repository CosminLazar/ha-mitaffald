use std::collections::HashMap;
use testcontainers::core::WaitFor;

const NAME: &str = "hivemq/hivemq-ce";
const TAG: &str = "latest";

pub struct HiveMQContainer {
    _env_vars: HashMap<String, String>,
    tag: String,
}

impl Default for HiveMQContainer {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("discovery.type".to_owned(), "single-node".to_owned());
        HiveMQContainer {
            _env_vars: env_vars,
            tag: TAG.to_owned(),
        }
    }
}

impl testcontainers::Image for HiveMQContainer {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.to_owned()
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![WaitFor::message_on_stdout("Started HiveMQ in")]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![1883]
    }
}
