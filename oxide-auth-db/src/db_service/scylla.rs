use scylla::{IntoTypedRows, Session, SessionBuilder, SessionConfig};
use scylla::transport::load_balancing::RoundRobinPolicy;
use std::sync::{Arc, mpsc};
use scylla::transport::errors::NewSessionError;
use super::client_data::StringfiedEncodedClient;
use std::io::{Error, ErrorKind};
use std::thread;

pub(crate) struct ScyllaHandler {
    handle: thread::JoinHandle<()>,
    input: mpsc::Sender<String>,
    output: Arc<mpsc::Receiver<StringfiedEncodedClient>>,
    db_name: String,
    db_table: String,
}

impl ScyllaHandler {
    pub fn new(db_nodes: Vec<String>, db_user: String, db_pwd: String, db_name: String, db_table: String) -> Self
    {
        let (tx1, rx1) = mpsc::channel::<String>();
        let (tx2, rx2) = mpsc::channel();
        let th = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build().unwrap();

            let _ = rt.block_on(async {
                let session = get_session(&db_nodes, db_user.as_str(), db_pwd.as_str()).await.map_err(|e|
                    Error::new(ErrorKind::Other, format!("{:?}", e)))?;
                let session = Arc::new(session);
                let rx = Arc::new(rx1);
                loop {
                    match rx.clone().recv() {
                        Ok(msg) => {
                            if msg.eq("stop") {
                                break
                            } else {
                                let client = get_app(
                                    session.clone(),
                                    db_name.as_str(),
                                    db_table.as_str(),
                                    client_id.as_str())
                                    .await.map_err(|err| Error::new(ErrorKind::Other, err.to_string()))?;
                                tx2.send(client).unwrap();
                            }
                        }
                        Err(err) => Err(Error::new(ErrorKind::Other, err.to_string()))
                    }
                }
                Ok(())
            });
        });
        ScyllaHandler {
            handle: th,
            input: tx1,
            output: Arc::new(rx2),
            db_name,
            db_table
        }
    }

    pub fn get_app(self, id: &str) -> Result<StringfiedEncodedClient, Error>
    {
        self.input.send(id.to_string()).map_err(|err| Error::new(ErrorKind::NotFound, err.to_string()))?;
        self.output.recv().map_err(|err| Error::new(ErrorKind::NotFound, err.to_string()))
    }

    pub fn stop(&self)
    {
        let _ = self.input.send("stop".to_string());
    }
}

impl Drop for ScyllaHandler {
    fn drop(&mut self) {
        let _ = self.handle.join();
    }
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

async fn get_app(session: Arc<Session>, db_name: &str, db_table: &str, client_id: &str) -> Result<StringfiedEncodedClient, Error>
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