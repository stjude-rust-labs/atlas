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
    Success,
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

    pub async fn pull_front(&self) -> sqlx::Result<Option<Task>> {
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

    pub async fn success(&self, id: Uuid) -> sqlx::Result<()> {
        sqlx::query!(
            "update tasks set status = $1 where id = $2",
            Status::Success as Status,
            id,
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

        assert!(queue.pull_front().await?.is_some());
        assert!(queue.pull_front().await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_success(pool: PgPool) -> sqlx::Result<()> {
        let queue = Queue::new(pool.clone());
        queue.push_back(Message::Noop).await?;
        let task = queue.pull_front().await?.unwrap();
        queue.success(task.id).await?;

        let actual_task = sqlx::query_as!(
            Task,
            r#"
            select
                id,
                status "status: Status",
                message "message: Json<Message>",
                created_at "created_at: Timestampz"
            from tasks
            where id = $1
            "#,
            task.id,
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(actual_task.status, Status::Success);

        Ok(())
    }
}
