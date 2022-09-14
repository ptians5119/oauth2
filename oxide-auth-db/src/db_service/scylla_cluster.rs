use oxide_auth::primitives::registrar::EncodedClient;
use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::{Arc};
use tokio::sync::Mutex;
use super::my_scylla::ScyllaHandler;
use std::str::FromStr;
use std::time::Duration;
use std::borrow::Borrow;

use super::client_data::StringfiedEncodedClient;
use crate::primitives::db_registrar::OauthClientDBRepository;


pub struct ScyllaClusterDataSource {
    session: ScyllaHandler,
}


impl ScyllaClusterDataSource {
    pub async fn new(nodes: Vec<&str>, username: &str, password: &str, db_name: &str, table_name: &str) -> anyhow::Result<Self> {
        let session = ScyllaHandler::new(
            nodes.iter().map(|x| x.to_string()).collect(),
            username.to_string(),
            password.to_string(),
            db_name.to_string(),
            table_name.to_string(),
        );
        Ok(ScyllaClusterDataSource {
            session
        })
    }

    pub fn regist(&self, client: EncodedClient) -> anyhow::Result<()> {
        Ok(())
    }
}


impl OauthClientDBRepository for ScyllaClusterDataSource {
    fn list(&self) -> anyhow::Result<Vec<EncodedClient>> {
        Err(anyhow::Error::msg("TODO"))
    }

    fn find_client_by_id(&self, id: &str) -> anyhow::Result<EncodedClient> {
        let client = self.session.get_app(id)?;
        Ok(client.to_encoded_client()?)
    }

    fn regist_from_encoded_client(&self, client: EncodedClient) -> anyhow::Result<()> {
        self.regist(client)
    }
}

