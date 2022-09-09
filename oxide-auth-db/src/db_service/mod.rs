mod client_data;

mod redis_isolate;
mod redis_cluster;
mod scylla_cluster;
mod redis_isolate_scylla_cluster;
mod redis_cluster_scylla_cluster;

use client_data::*;
use std::io::{Error, ErrorKind};
use scylla::{SessionBuilder, FromRow, Session, IntoTypedRows};
use std::{sync::{Arc, mpsc}, thread};
use tokio::runtime::Handle;
use tokio::sync::Mutex;

cfg_if::cfg_if! {
    if #[cfg(feature = "redis-isolate")] {
        use redis_isolate::RedisDataSource ;
        pub type DataSource = RedisDataSource;
    }else if  #[cfg(feature = "redis-cluster")]{
        use redis_cluster::RedisClusterDataSource ;
        pub type DataSource = RedisClusterDataSource;
    }else if #[cfg(feature = "scylla-cluster")] {
        use scylla_cluster::ScyllaClusterDataSource ;
        pub type DataSource = ScyllaClusterDataSource;
    }else if #[cfg(feature = "redis-isolate-scylla-cluster")]{
        use redis_isolate_scylla_cluster::RedisIsolateScyllaCluster;
        pub type DataSource = RedisIsolateScyllaCluster;
    }else if #[cfg(feature = "redis-cluster-scylla-cluster")]{
        use redis_cluster_scylla_cluster::RedisClusterScyllaCluster;
        pub type DataSource = RedisClusterScyllaCluster;
    }
}

pub fn get_client(session: Arc<Mutex<Session>>, info: (String, String, &str)) -> Result<StringfiedEncodedClient, Error> {
    let handle = Handle::current();
    let (tx, rx) = mpsc::channel();
    let th = thread::spawn(move || {
        handle.spawn(async move {
            let smt = r#"SELECT client_id, client_secret, redirect_uri, additional_redirect_uris
                    , scopes as default_scope FROM xbot.apps where client_id = '380235020360617984'"#;
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
                        tx.send(Err(Error::new(ErrorKind::Other, "xxx2"))).unwrap();
                        return
                    }
                };
                tx.send(Ok(client)).unwrap();
                return
            }
            tx.send(Err(Error::new(ErrorKind::NotFound, "no rows"))).unwrap();
        });
    });
    let client = match rx.recv() {
        Ok(c) => c,
        Err(e) => Err(Error::new(ErrorKind::Other, format!("{:?}", e)))
    };
    th.join().unwrap();
    client
}