use uuid::Timestamp;

pub type ReviewId = uuid::Uuid;

pub fn create_review_id(timestamp: impl Into<Timestamp>) -> ReviewId {
    ReviewId::new_v7(timestamp.into())
}
