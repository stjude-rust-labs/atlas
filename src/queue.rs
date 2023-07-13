use serde::{Deserialize, Serialize};
use sqlx::{types::Json, PgPool};
use uuid::Uuid;

use crate::server::types::Timestampz;

pub struct Queue {
    pool: PgPool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, sqlx::Type)]
#[repr(i16)]
pub enum Status {
    Queued,
    Running,
    Failed,
}

pub struct Task {
    pub id: Uuid,
    pub status: Status,
    pub message: Json<Message>,
    pub created_at: Timestampz,
}

#[derive(Deserialize, Serialize)]
pub enum Message {
    Noop,
}

impl Queue {
    pub fn new(pool: PgPool) -> Self {
        Queue { pool }
    }

    pub async fn pop_front(&self) -> sqlx::Result<Option<Task>> {
        sqlx::query_as!(
            Task,
            r#"
            update tasks
            set status = $1
            where id = (
                select id
                from tasks
                where status = $2
                order by id
                for update skip locked
                limit 1
            )
            returning
                id,
                status "status: Status",
                message "message: Json<Message>",
                created_at "created_at: Timestampz"
        "#,
            Status::Running as Status,
            Status::Queued as Status,
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn push_back(&self, message: Message) -> sqlx::Result<()> {
        let id = Uuid::new_v4();
        let message = Json(message);

        sqlx::query!(
            r#"insert into tasks (id, status, message) values ($1, $2, $3)"#,
            id,
            Status::Queued as Status,
            message as Json<Message>,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_self(pool: PgPool) -> sqlx::Result<()> {
        let queue = Queue::new(pool);

        queue.push_back(Message::Noop).await?;

        assert!(queue.pop_front().await?.is_some());
        assert!(queue.pop_front().await?.is_none());

        Ok(())
    }
}
