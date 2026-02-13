use serde::{Serialize, Deserialize};
use crate::tensor::SSAUTensor;

#[derive(Serialize, Deserialize, Debug)]
pub struct SSAUPacket {
    pub node_id: String,
    pub tensor: SSAUTensor,
    pub timestamp: u64,
}

impl SSAUPacket {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Ошибка сериализации пакета")
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).expect("Ошибка десериализации пакета")
    }
}
