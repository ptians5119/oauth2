mod client_data;

mod redis_isolate;
mod redis_cluster;
mod scylla_cluster;
mod redis_isolate_scylla_cluster;
mod redis_cluster_scylla_cluster;

use client_data::*;


// #[cfg(feature = "redis-isolate")]
// pub mod redis_isolate;
// #[cfg(feature = "redis-isolate")]
// use redis_isolate::RedisDataSource ;
// #[cfg(feature = "redis-isolate")]
// pub type DataSource = RedisDataSource;

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



//
// cfg_if::cfg_if! {
//     if #[cfg(feature = "redis-cluster")]{
//         pub mod redis_cluster;
//         use redis_cluster::RedisClusterDataSource ;
//         pub type DataSource = RedisClusterDataSource;
//     }
// }
//
//
//
//
// cfg_if::cfg_if! {
//     if #[cfg(feature = "scylla-cluster")] {
//         pub mod scylla_cluster;
//         use scylla_cluster::ScyllaClusterDataSource ;
//         pub type DataSource = ScyllaClusterDataSource;
//     }
// }
//
//
//
//
// cfg_if::cfg_if! {
//     if #[cfg(feature = "redis-isolate-scylla-cluster")]{
//         pub mod redis_isolate_scylla_cluster;
//         use redis_isolate_scylla_cluster::RedisIsolateScyllaCluster;
//         pub type DataSource = RedisIsolateScyllaCluster;
//     }
// }
//
//
//
// cfg_if::cfg_if! {
//     if #[cfg(feature = "redis-cluster-scylla-cluster")]{
//         pub mod redis_cluster_scylla_cluster;
//         use redis_cluster_scylla_cluster::RedisClusterScyllaCluster;
//         pub type DataSource = RedisClusterScyllaCluster;
//     }
// }


