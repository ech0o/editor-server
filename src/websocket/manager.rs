use std::collections::HashMap;
use tokio::sync::{Mutex,mpsc};
use uuid::Uuid;
use crate::kafka::JobEvent;

pub struct WsManager {
    connections:Mutex<HashMap<Uuid,mpsc::Sender<JobEvent>>>
}

impl WsManager {
    pub fn new() -> WsManager {
        Self{
            connections:Mutex::new(HashMap::new())
        }
    }

    pub async fn add(
        &self,
        job_id:Uuid,
        sender:mpsc::Sender<JobEvent>,
    ){
        self.connections.lock().await.insert(job_id, sender);
    }

    pub async fn remove(&self, job_id:Uuid){
        self.connections.lock().await.remove(&job_id);
    }

    pub async fn send(&self, event:JobEvent){
        let connections = self.connections.lock().await;
        if let Some(sender) = connections.get(&event.job_id) {
            let _ = sender.send(event).await;
        }
    }
}