use oxide_auth::primitives::registrar::EncodedClient;
use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::Arc;
use tokio::sync::Mutex;

use std::str::FromStr;
use std::time::Duration;
use std::borrow::Borrow;

use super::StringfiedEncodedClient;
use crate::primitives::db_registrar::OauthClientDBRepository;


pub struct ScyllaClusterDataSource {
    session: Arc<Mutex<Session>>,
    db_name: String,
    table_name: String,
}


impl ScyllaClusterDataSource {
    pub async fn new(nodes: Vec<&str>, username: &str, password: &str, db_name: &str, table_name: &str) -> anyhow::Result<Self> {
        let session = SessionBuilder::new()
            .known_nodes(&nodes)
            .user(username, password)
            .load_balancing(Arc::new(RoundRobinPolicy::new()))
            .build()
            .await
            .unwrap();;

        Ok(ScyllaClusterDataSource {
            session: Arc::new(Mutex::new(session)),
            db_name: db_name.to_string(),
            table_name: table_name.to_string(),
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
        let session = self.session.clone();
        let client = super::get_client(session, self.db_name.clone(), self.table_name.clone(), id)?;
        Ok(client.to_encoded_client()?)
    }

    fn regist_from_encoded_client(&self, client: EncodedClient) -> anyhow::Result<()> {
        self.regist(client)
    }
}

