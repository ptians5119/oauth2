use oxide_auth::primitives::registrar::EncodedClient;
use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::{Arc};
use tokio::sync::Mutex;

use std::str::FromStr;
use std::time::Duration;
use std::borrow::Borrow;

use super::client_data::StringfiedEncodedClient;
use crate::primitives::db_registrar::OauthClientDBRepository;


pub struct ScyllaClusterDataSource {
    // session: Arc<Mutex<Session>>,
    db_nodes: Vec<String>,
    db_user: String,
    db_pwd: String,
    db_name: String,
    table_name: String,
}


impl ScyllaClusterDataSource {
    pub async fn new(nodes: Vec<&str>, username: &str, password: &str, db_name: &str, table_name: &str) -> anyhow::Result<Self> {
        // let session = SessionBuilder::new()
        //     .known_nodes(&nodes)
        //     .user(username, password)
        //     .load_balancing(Arc::new(RoundRobinPolicy::new()))
        //     .build()
        //     .await
        //     .unwrap();;

        Ok(ScyllaClusterDataSource {
            // session: Arc::new(Mutex::new(session)),
            db_nodes: nodes.iter().map(|x| x.to_string()).collect(),
            db_user: username.to_string(),
            db_pwd: password.to_string(),
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
        let (tx, rx) = std::sync::mpsc::channel();
        let nodes = self.db_nodes.clone();
        let user = self.db_user.clone();
        let pwd = self.db_pwd.clone();
        let db = self.db_name.clone();
        let table = self.table_name.clone();
        let id = id.to_string();
        let th = std::thread::spawn(move || {
            let client = super::scylla::handle(
                nodes,
                user,
                pwd,
                db,
                table,
                id
            );
            let _ = tx.send(client);
        });
        let _ = th.join();
        let client = rx.recv()??;
        Ok(client.to_encoded_client()?)
    }

    fn regist_from_encoded_client(&self, client: EncodedClient) -> anyhow::Result<()> {
        self.regist(client)
    }
}

