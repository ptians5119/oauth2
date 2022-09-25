mod client_data;
mod redis_isolate;
mod redis_cluster;
mod scylla_cluster;
mod redis_isolate_scylla_cluster;
mod redis_cluster_scylla_cluster;

use client_data::*;
use std::io::{Error, ErrorKind};
use scylla::{Session, IntoTypedRows};
use std::sync::{Arc, mpsc};
use std::rc::Rc;
use tokio::sync::Mutex;
use tokio::runtime::Handle;
use std::thread;
use std::time::Duration;

use redis_cluster_scylla_cluster::RedisClusterScyllaCluster;
pub type DataSource = RedisClusterScyllaCluster;

pub fn get_client(session: Arc<Mutex<Rc<Session>>>, db_name: String, table_name: String, id: String) -> Result<StringfiedEncodedClient, Error> {


    let handle = Handle::current();
    let (tx, rx) = mpsc::channel();
    let th = thread::spawn(move || {
        handle.spawn(async move {
            let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris
                    , scopes as default_scope FROM {}.{} where client_id = '{}'", db_name, table_name, id);
            let res = match session.lock().await.query(smt.clone(), &[]).await {
                Ok(r) => r,
                Err(e) => {
                    tx.send(Err(Error::new(ErrorKind::Other, format!("{:?}", e)))).unwrap();
                    return
                }
            };
            for row in res.rows.unwrap()
                .into_typed::<StringfiedEncodedClient>() {
                let client = match row {
                    Ok(r) => r,
                    Err(_e) => {
                        tx.send(Err(Error::new(ErrorKind::Other, _e.to_string()))).unwrap();
                        return
                    }
                };
                debug!("get client");
                tx.send(Ok(client)).unwrap();
                return
            }
            tx.send(Err(Error::new(ErrorKind::NotFound, "no rows"))).unwrap();
        });
    });
    th.join().unwrap();
    let client = match rx.recv_timeout(Duration::from_millis(500)) {
        Ok(c) => c,
        Err(err) => Err(Error::new(ErrorKind::NotFound, format!("cql handle timeout {}", err)))
    };
    client
}