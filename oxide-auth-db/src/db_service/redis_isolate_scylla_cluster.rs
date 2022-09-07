use oxide_auth::primitives::registrar::EncodedClient;
use redis::{Commands, RedisError, ErrorKind, Client, ConnectionInfo, ToRedisArgs};

use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::Arc;

use std::str::FromStr;
use url::Url;

use crate::primitives::db_registrar::OauthClientDBRepository;
use super::StringfiedEncodedClient;

/// redis datasource to Client entries.
pub struct RedisIsolateScyllaCluster {
    scylla_session: Session,
    redis_client: Client,
    redis_prefix: String,
    db_name: String,
    db_table: String,
}


impl RedisIsolateScyllaCluster {
    pub async fn new(redis_nodes: Vec<&str>, redis_prefix: &str, redis_pwd: Option<&str>, db_nodes: Vec<&str>, db_user: &str, db_pwd: &str, db_name: &str, db_table: &str) -> anyhow::Result<Self> {
        let mut info = ConnectionInfo::from_str(redis_nodes[0]).map_err(|err|{
            error!("{}", err.to_string());
            err
        })?;

        if redis_pwd.is_some(){
            info.passwd = redis_pwd.map(|s|s.to_string());
        }
        let client = Client::open(info).map_err(|err|{
            error!("{}", err.to_string());
            err
        })?;

        let session = SessionBuilder::new()
            .known_nodes(&db_nodes)
            .user(db_user, db_pwd)
            .load_balancing(Arc::new(RoundRobinPolicy::new()))
            .build()
            .await
            .unwrap();

        Ok(RedisIsolateScyllaCluster {
            scylla_session: session,
            redis_client: client,
            redis_prefix: redis_prefix.to_string(),
            db_name: db_name.to_string(),
            db_table: db_table.to_string(),
        })
    }
    pub fn regist_to_cache(&self, detail: &StringfiedEncodedClient) -> anyhow::Result<()> {
        let mut connect = self.redis_client.get_connection()?;
        let client_str = serde_json::to_string(&detail)?;
        connect.set_ex(&(self.redis_prefix.to_owned() + &detail.client_id), client_str, 3600)?;
        Ok(())
    }

    pub fn delete_from_cache(&self, client_id: &str) -> anyhow::Result<()> {
        let mut connect = self.redis_client.get_connection()?;
        connect.del(&(self.redis_prefix.to_owned() + client_id))?;
        Ok(())
    }

}

impl OauthClientDBRepository for RedisIsolateScyllaCluster {
    fn list(&self) -> anyhow::Result<Vec<EncodedClient>> {
        let mut encoded_clients: Vec<EncodedClient> = vec![];
        let mut r = self.redis_client.get_connection()?;
        let keys = r.keys::<&str, Vec<String>>(&self.redis_prefix)?;
        for key in keys {
            let clients_str = r.get::<String, String>(key)?;
            let stringfied_client = serde_json::from_str::<StringfiedEncodedClient>(&clients_str)?;
            encoded_clients.push(stringfied_client.to_encoded_client()?);
        }
        Ok(encoded_clients)
    }

    fn find_client_by_id(&self, id: &str) -> anyhow::Result<EncodedClient> {
        let mut r = self.redis_client.get_connection()?;
        let client_str = match r.get::<&str, String>(&(self.redis_prefix.to_owned() + id)){
            Ok(v) => {v}
            Err(err) => {
                error!("{}", err.to_string());
                "".to_string()
            }
        };
        if &client_str == ""{
            futures::executor::block_on(async {
                let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris\
                    , scopes as default_scope FROM {}.{} where client_id = {}", self.db_name, self.db_table, id);
                if let Some(rows) = self.scylla_session.query(smt.clone(), &[]).await.map_err(|err|{
                    error!("failed to excute smt={} with err={:?}", smt, err);
                    anyhow::Error::from(err)
                })?.rows {
                    for row in rows.into_typed::<StringfiedEncodedClient>() {
                        let client = row?;
                        return Ok(client.to_encoded_client()?)
                    }
                }
                Err(anyhow::Error::msg("Not Found"))
            })
        }else{
            let stringfied_client = serde_json::from_str::<StringfiedEncodedClient>(&client_str)?;
            Ok(stringfied_client.to_encoded_client()?)
        }

    }

    fn regist_from_encoded_client(&self, client: EncodedClient) -> anyhow::Result<()> {
        let detail = StringfiedEncodedClient::from_encoded_client(&client);
        self.regist_to_cache(&detail)
    }
}
