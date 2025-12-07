// Trait для репозитория тикетов

use crate::ports::{entities::Ticket, error::Error};

pub trait TicketRepository: Send + Sync {
    /// Создать новый тикет
    fn create(
        &self,
        session_id: &str,
        service_url: &str,
        expires_at: u64,
    ) -> Result<Ticket, Error>;

    /// Получить тикет по его ID
    fn get(&self, ticket_id: &str) -> Result<Ticket, Error>;

    /// Пометить тикет как использованный (consumed)
    fn consume(&self, ticket_id: &str) -> Result<(), Error>;

    /// Удалить тикет
    fn delete(&self, ticket_id: &str) -> Result<(), Error>;

    /// Проверить существование тикета
    fn exists(&self, ticket_id: &str) -> Result<bool, Error>;
}
