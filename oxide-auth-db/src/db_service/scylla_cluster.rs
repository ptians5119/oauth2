use oxide_auth::primitives::registrar::EncodedClient;
use cdrs::authenticators::StaticPasswordAuthenticator;
use cdrs::cluster::session::{new as new_session, Session};
use cdrs::cluster::{ClusterTcpConfig, NodeTcpConfigBuilder, TcpConnectionPool};
use cdrs::load_balancing::RoundRobin;
use cdrs::query::*;
use cdrs::types::prelude::*;
use cdrs::frame::IntoBytes;
use cdrs::types::from_cdrs::FromCDRSByName;

use std::str::FromStr;
use std::time::Duration;
use std::borrow::Borrow;

use super::StringfiedEncodedClient;
use crate::primitives::db_registrar::OauthClientDBRepository;


type CurrentSession = Session<RoundRobin<TcpConnectionPool<StaticPasswordAuthenticator>>>;


pub struct ScyllaClusterDataSource {
    session: CurrentSession,
    db_name: String,
    table_name: String,
}


impl ScyllaClusterDataSource {
    pub fn new(nodes: Vec<&str>, username: &str, password: &str, db_name: &str, table_name: &str) -> anyhow::Result<Self> {
        let auth = StaticPasswordAuthenticator::new(username, password);
        let mut configs = vec![];

        for n in nodes {
            let node = NodeTcpConfigBuilder::new(n, auth.clone()).build();
            configs.push(node);
        }
        let session = new_session(&ClusterTcpConfig(configs), RoundRobin::new()).map_err(|err|{
            error!("{}", err.to_string());
            err
        })?;

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
        let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris, scopes as default_scope FROM {}.{} where client_id = ?", self.db_name, self.table_name);
        let r = self.session.query_with_values(smt, query_values!(id))?
            .get_body()?
            .into_rows().ok_or(anyhow::Error::msg("Record Not Found"))?;
        return if r.len() > 0 {
            let b: StringfiedEncodedClient = StringfiedEncodedClient::try_from_row(r.get(0).unwrap().to_owned())?;
            let client = b.to_encoded_client()?;
            Ok(client)
        } else {
            Err(anyhow::Error::msg("Not Found"))
        };
    }

    fn regist_from_encoded_client(&self, client: EncodedClient) -> anyhow::Result<()> {
        self.regist(client)
    }
}

