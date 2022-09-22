mod client_data;
mod redis_isolate;
mod redis_cluster;
mod scylla_cluster;
mod redis_isolate_scylla_cluster;
mod redis_cluster_scylla_cluster;

use client_data::*;
use std::io::{Error, ErrorKind};
use scylla::{SessionBuilder, FromRow, Session, IntoTypedRows};
use std::{sync::{Arc, mpsc}, thread, time::Duration};
use tokio::runtime::Handle;
use tokio::sync::{Mutex, oneshot};
use futures::executor::block_on;
use actix_rt::System;

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

pub fn get_client(session: Arc<Mutex<Session>>, db_name: String, table_name: String, id: String) -> Result<StringfiedEncodedClient, Error> {
    futures::executor::block_on(async {
        println!("input block_on");
        let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris
                    , scopes as default_scope FROM {}.{} where client_id = '{}'", db_name, table_name, id);
        let res = {
            let ss = session.lock().await;
            match ss.query(smt.clone(), &[]).await {
                Ok(r) => r,
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
                    return Err(Error::new(ErrorKind::Other, "parse client error"))
                }
            };
            debug!("get client");
            return Ok(client)
        }
        Err(Error::new(ErrorKind::Other, "no client"))
    })
    // let (tx, rx) = oneshot::channel();
    // println!("oxide-db: inside");
    // System::current().arbiter().spawn(async move {
    //     let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris
    //             , scopes as default_scope FROM {}.{} where client_id = '{}'", db_name, table_name, id);
    //     let res = match session.lock().await.query(smt.clone(), &[]).await {
    //         Ok(r) => r,
    //         Err(e) => {
    //             tx.send(Err(Error::new(ErrorKind::Other, format!("{:?}", e)))).unwrap();
    //             return
    //         }
    //     };
    //     println!("oxide-db: get row");
    //     for row in res.rows.unwrap()
    //         .into_typed::<StringfiedEncodedClient>() {
    //         let client = match row {
    //             Ok(r) => r,
    //             Err(_e) => {
    //                 tx.send(Err(Error::new(ErrorKind::Other, "xxx2"))).unwrap();
    //                 return
    //             }
    //         };
    //         debug!("get client");
    //         tx.send(Ok(client)).unwrap();
    //         return
    //     }
    //     tx.send(Err(Error::new(ErrorKind::NotFound, "no rows"))).unwrap();
    // });
    // let client = match block_on(async {
    //     rx.await
    // }) {
    //     Ok(c) => c,
    //     Err(err) => Err(Error::new(ErrorKind::NotFound, err.to_string()))
    // };
    // client
}