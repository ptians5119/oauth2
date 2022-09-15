use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::{Arc, mpsc, Mutex};
use std::rc::Rc;
use scylla::transport::errors::NewSessionError;
use super::client_data::StringfiedEncodedClient;
use std::io::{Error, ErrorKind};
use std::thread;

pub struct ScyllaHandler {
    session: Session,
    db_name: String,
    db_table: String
}

impl ScyllaHandler {
    pub async fn new(db_nodes: Vec<String>, db_user: String, db_pwd: String, db_name: String, db_table: String) -> Self
    {
        let session = ScyllaHandler::get_session(
            &db_nodes,
            db_user.as_str(),
            db_pwd.as_str()
        ).await.expect("create oauth session error");
        ScyllaHandler {
            session,
            db_name,
            db_table
        }
    }

    pub fn get_app(&self, id: &str) -> Result<StringfiedEncodedClient, Error>
    {
        futures::executor::block_on(async {
            self.get_app_by_db(id).await
        })
    }

    async fn get_session(db_nodes: &Vec<String>, db_user: &str, db_pwd: &str) -> Result<Session, NewSessionError>
    {
        SessionBuilder::new()
            .known_nodes(&db_nodes)
            .user(db_user, db_pwd)
            .load_balancing(Arc::new(RoundRobinPolicy::new()))
            .build()
            .await
    }

    async fn get_app_by_db(&self, client_id: &str) -> Result<StringfiedEncodedClient, Error>
    {
        let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris
                    , scopes as default_scope FROM {}.{} where client_id = '{}'", self.db_name, self.db_table, client_id);
        let res = {
            match self.session.query(smt.clone(), &[]).await {
                Ok(r) => {
                    r
                },
                Err(e) => {
                    return Err(Error::new(ErrorKind::Other, format!("{:?}", e)))
                }
            }
        };
        for row in res.rows.unwrap()
            .into_typed::<StringfiedEncodedClient>() {
            let client = match row {
                Ok(r) => r,
                Err(_e) => {
                    return Err(Error::new(ErrorKind::Other, "match row error"))
                }
            };
            return Ok(client)
        }
        Err(Error::new(ErrorKind::NotFound, "no rows"))
    }
}