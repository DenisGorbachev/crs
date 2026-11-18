use crate::Server;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedServer = Arc<Mutex<Server>>;
