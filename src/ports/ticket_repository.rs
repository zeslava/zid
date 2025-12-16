// Trait для репозитория тикетов

use crate::ports::{entities::Ticket, error::Error};

pub trait TicketRepository: Send + Sync {
    /// Создать новый тикет
    fn create(&self, session_id: &str, service_url: &str, expires_at: u64)
    -> Result<Ticket, Error>;

    /// Получить тикет по его ID
    fn get(&self, ticket_id: &str) -> Result<Ticket, Error>;

    /// Удалить тикет
    fn delete(&self, ticket_id: &str) -> Result<(), Error>;
}
