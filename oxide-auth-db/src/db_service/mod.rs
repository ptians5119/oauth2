mod client_data;
mod redis_isolate;
mod redis_cluster;
mod scylla_cluster;
mod redis_isolate_scylla_cluster;
mod redis_cluster_scylla_cluster;

use client_data::*;
use std::io::{Error, ErrorKind};
use scylla::{Session, IntoTypedRows};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::executor::block_on;

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
    block_on(async {
        let smt = format!("SELECT client_id, client_secret, redirect_uri, additional_redirect_uris
                    , scopes as default_scope FROM {}.{} where client_id = '{}'", db_name, table_name, id);
        let res = {
            println!("inside");
            let ss = session.lock().await;
            println!("get session");
            match ss.query(smt.clone(), &[]).await {
                Ok(r) => r,
                Err(e) => {
                    return Err(Error::new(ErrorKind::Other, format!("{:?}", e)))
                }
            }
        };
        println!("get rows");
        for row in res.rows.unwrap()
            .into_typed::<StringfiedEncodedClient>() {
            let client = match row {
                Ok(r) => r,
                Err(_e) => {
                    return Err(Error::new(ErrorKind::Other, "parse client error"))
                }
            };
            return Ok(client)
        }
        Err(Error::new(ErrorKind::Other, "no client"))
    })
}