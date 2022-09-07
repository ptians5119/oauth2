use oxide_auth::primitives::registrar::EncodedClient;
use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::Arc;

use std::str::FromStr;
use std::time::Duration;
use std::borrow::Borrow;

use super::StringfiedEncodedClient;
use crate::primitives::db_registrar::OauthClientDBRepository;


pub struct ScyllaClusterDataSource {
    session: Session,
    db_name: String,
    table_name: String,
}


impl ScyllaClusterDataSource {
    pub fn new(nodes: Vec<&str>, username: &str, password: &str, db_name: &str, table_name: &str) -> anyhow::Result<Self> {
        let session = SessionBuilder::new()
            .known_nodes(&nodes)
            .user(username, password)
            .load_balancing(Arc::new(RoundRobinPolicy::new()))
            .build()
            .await
            .unwrap();;

        Ok(ScyllaClusterDataSource {
            session,
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
        let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris, scopes as default_scope FROM {}.{} where client_id = {}", self.db_name, self.db_table, id);
        if let Some(rows) = self.session.query(smt.clone(), &[]).await.map_err(|err|{
            error!("failed to excute smt={} with err={:?}", smt, err);
            anyhow::Error::from(err)
        })?.rows {
            for row in rows.into_typed::<StringfiedEncodedClient>() {
                let c = row?;
                let client = c.to_encoded_client()?;
                return Ok(client);
            }
        }
        Err(anyhow::Error::msg("Not Found"))
    }

    fn regist_from_encoded_client(&self, client: EncodedClient) -> anyhow::Result<()> {
        self.regist(client)
    }
}

