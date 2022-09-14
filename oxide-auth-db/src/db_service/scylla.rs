use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::Arc;
use scylla::transport::errors::NewSessionError;
use super::client_data::StringfiedEncodedClient;
use std::io::{Error, ErrorKind};

async fn get_session(db_nodes: &Vec<String>, db_user: &str, db_pwd: &str) -> Result<Session, NewSessionError>
{
    SessionBuilder::new()
        .known_nodes(&db_nodes)
        .user(db_user, db_pwd)
        .load_balancing(Arc::new(RoundRobinPolicy::new()))
        .build()
        .await
}

async fn get_app(session: Session, db_name: &str, db_table: &str, client_id: &str) -> Result<StringfiedEncodedClient, Error>
{
    let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris
                    , scopes as default_scope FROM {}.{} where client_id = '{}'", db_name, db_table, client_id);
    let res = {
        match session.query(smt.clone(), &[]).await {
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

pub(crate) fn handle(db_nodes: Vec<String>, db_user: String, db_pwd: String, db_name: String, db_table: String, id: String) -> Result<StringfiedEncodedClient, Error>
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build().unwrap();

    let res = rt.block_on(async {
        let session = get_session(&db_nodes, db_user.as_str(), db_pwd.as_str()).await.map_err(|e|
            Error::new(ErrorKind::Other, format!("{:?}", e)))?;
        get_app(session, db_name.as_str(), db_table.as_str(), id.as_str()).await
    });

    res
}