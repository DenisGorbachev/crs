use uuid::Timestamp;

pub type MessageId = uuid::Uuid;

pub fn create_message_id(timestamp: impl Into<Timestamp>) -> MessageId {
    MessageId::new_v7(timestamp.into())
}
