use redis::Commands;
use serde::{Deserialize, Serialize};

use crate::ports::{entities::Ticket, error::Error, ticket_repository::TicketRepository};

pub struct RedisTicketRepository {
    client: redis::Client,
}

impl RedisTicketRepository {
    pub fn new(client: redis::Client) -> Self {
        RedisTicketRepository { client }
    }
}

impl TicketRepository for RedisTicketRepository {
    fn create(
        &self,
        session_id: &str,
        service_url: &str,
        expires_at: u64,
    ) -> Result<Ticket, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let ticket_id = uuid::Uuid::new_v4().to_string();

        let ticket = Ticket {
            id: ticket_id.clone(),
            session_id: session_id.to_string(),
            service_url: service_url.to_string(),
            expires_at,
            consumed: false,
        };

        let key = format!("ticket:id:{}", ticket_id);
        let dto: TicketDTO = ticket.clone().into();
        let serialized =
            serde_json::to_string(&dto).map_err(|e| Error::Repository(e.to_string()))?;

        // Calculate TTL
        let ttl = if expires_at > 0 {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            expires_at.saturating_sub(current_time).max(1)
        } else {
            300 // Default 5 minutes
        };

        let _: () = conn
            .set_ex(&key, &serialized, ttl)
            .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(ticket)
    }

    fn get(&self, ticket_id: &str) -> Result<Ticket, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let key = format!("ticket:id:{}", ticket_id);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        match res {
            Some(data) => {
                let dto: TicketDTO =
                    serde_json::from_str(&data).map_err(|e| Error::Repository(e.to_string()))?;
                Ok(dto.into())
            }
            None => Err(Error::TicketNotFound),
        }
    }

    fn delete(&self, ticket_id: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let key = format!("ticket:id:{}", ticket_id);
        let _: () = conn
            .del(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct TicketDTO {
    id: String,
    session_id: String,
    service_url: String,
    expires_at: u64,
    consumed: bool,
}

impl From<Ticket> for TicketDTO {
    fn from(ticket: Ticket) -> Self {
        TicketDTO {
            id: ticket.id,
            session_id: ticket.session_id,
            service_url: ticket.service_url,
            expires_at: ticket.expires_at,
            consumed: ticket.consumed,
        }
    }
}

impl From<TicketDTO> for Ticket {
    fn from(dto: TicketDTO) -> Self {
        Ticket {
            id: dto.id,
            session_id: dto.session_id,
            service_url: dto.service_url,
            expires_at: dto.expires_at,
            consumed: dto.consumed,
        }
    }
}
