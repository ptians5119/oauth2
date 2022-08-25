use oxide_auth::primitives::registrar::EncodedClient;
use redis::{Commands, RedisError, ErrorKind, Client, ConnectionInfo};
use url::Url;

use std::str::FromStr;

use crate::primitives::db_registrar::OauthClientDBRepository;
use super::StringfiedEncodedClient;


/// redis datasource to Client entries.
#[derive(Debug, Clone)]
pub struct RedisDataSource {
    redis_client: Client,
    redis_prefix: String,
}


impl RedisDataSource {
    pub fn new(url: &str, redis_prefix: &str, password: Option<String>) -> Result<Self, RedisError> {
        let mut info = ConnectionInfo::from_str(url).map_err(|err|{
            error!("{}", err.to_string());
            err
        })?;
        info.passwd = password;
        let redis_client = Client::open(info).map_err(|err|{
            error!("{}", err.to_string());
            err
        })?;
        Ok(RedisDataSource {
            redis_client,
            redis_prefix: redis_prefix.to_string(),
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



impl OauthClientDBRepository for RedisDataSource {
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
        let client_str = r.get::<&str, String>(&(self.redis_prefix.to_owned() + id))?;
        let stringfied_client = serde_json::from_str::<StringfiedEncodedClient>(&client_str)?;
        Ok(stringfied_client.to_encoded_client()?)
    }

    fn regist_from_encoded_client(&self, client: EncodedClient) -> anyhow::Result<()> {
        let detail = StringfiedEncodedClient::from_encoded_client(&client);
        self.regist_to_cache(&detail)
    }
}
