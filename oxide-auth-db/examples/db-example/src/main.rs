#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod support;

use actix::{Actor, Addr, Context, Handler};
use actix_rt;
use actix_web::{middleware::Logger, web, App, HttpRequest, HttpServer};
use oxide_auth::{
    endpoint::{Endpoint, OwnerConsent, OwnerSolicitor, Solicitation},
    frontends::simple::endpoint::{ErrorInto, FnSolicitor, Generic, Vacant},
    primitives::prelude::{AuthMap, RandomGenerator, Scope, TokenMap},
};

use oxide_auth_actix::{
    Authorize, OAuthMessage, OAuthOperation, OAuthRequest, OAuthResource, OAuthResponse, Refresh,
    Resource, Token, WebError,
};
use oxide_auth_db::primitives::db_registrar::DBRegistrar;
use oxide_auth_db::db_service::*;

use std::{thread, env};
use std::io::Write;
use std::collections::hash_map::HashMap;
use std::time::Duration;
use oxide_auth_db::db_service::DataSource;

static DENY_TEXT: &str = "<html>
This page should be accessed via an oauth token from the client in the example. Click
<a href=\"http://localhost:8020/authorize?response_type=code&client_id=LocalClient&state=12345\">
here</a> to begin the authorization process.
</html>
";


enum Extras {
    AuthGet,
    AuthPost(String),
    Nothing,
}

async fn get_authorize(
    (req, state): (OAuthRequest, web::Data<Addr<State>>),
) -> Result<OAuthResponse, WebError> {
    debug!("/get_authorize {:?}", req);
    // GET requests should not mutate server state and are extremely
    // vulnerable accidental repetition as well as Cross-Site Request
    // Forgery (CSRF).
    let response = state.send(Authorize(req).wrap(Extras::AuthGet)).await.map_err(|err| {
        error!("get_authorize {:?}", err);
        err
    }).unwrap();

    debug!("/get_authorize {:?}\n\n", response);

    response
}

async fn post_authorize(
    (r, req, state): (HttpRequest, OAuthRequest, web::Data<Addr<State>>),
) -> Result<OAuthResponse, WebError> {
    debug!("/post_authorize {:?} {:?}", r, req);
    // Some authentication should be performed here in production cases
    let res = state
        .send(Authorize(req).wrap(Extras::AuthPost(r.query_string().to_owned())))
        .await.map_err(|err| {
        error!("post_authorize {:?}", err);
        err
    })?;
    debug!("/post_authorize {:?}\n\n", res);
    res
}

async fn token((req, state): (OAuthRequest, web::Data<Addr<State>>)) -> Result<OAuthResponse, WebError> {
    debug!("/token {:?}", req);
    let r = Token(req).wrap(Extras::Nothing);
    let res = state.send(r).await.map_err(|err| {
        error!("token err = {:?}", err);
        err
    })?;
    debug!("/token res = {:?}\n\n", res);
    res
}

async fn refresh(
    (req, state): (OAuthRequest, web::Data<Addr<State>>),
) -> Result<OAuthResponse, WebError> {
    debug!("/refresh {:?}", req);
    let res = state.send(Refresh(req).wrap(Extras::Nothing)).await.map_err(|err| {
        error!("refresh {:?}", err);
        err
    })?;
    debug!("/refresh {:?}\n\n", res);
    res
}

async fn index(
    (req, state): (HttpRequest, web::Data<Addr<State>>),
) -> Result<OAuthResponse, WebError> {
    debug!("/index {:?}\n\n", req);
    let req = OAuthResource::new(&req).unwrap();
    let res = match state
        .send(Resource(req.into_request()).wrap(Extras::Nothing))
        .await.map_err(|err| {
        error!("index {:?}", err);
        err
    })?
    {
        Ok(_grant) => {
            Ok(OAuthResponse::ok()
                .content_type("text/plain")?
                .body("Hello world!"))
        }
        Err(Ok(e)) => Ok(e.body(DENY_TEXT)),
        Err(Err(e)) => Err(e),
    };
    debug!("/index {:?}\n\n", res);
    res
}

async fn start_browser() -> () {
    let _ = thread::spawn(support::open_in_browser);
}

/// Example of a main function of an actix-web server supporting oauth.
#[actix_web::main]
async fn main() {
    env_logger::Builder::new().format(|buf, record| {
        writeln!(buf, "[{}] {}:{} {}", record.level(), record.module_path().unwrap_or_default(), record.line().unwrap_or(0), record.args())
    }).parse_filters("debug,tokio_reactor=info,hyper=info").init();

    std::env::set_var("REDIS_URL", "redis://129.204.249.76:30379/2");
    std::env::set_var("MAX_POOL_SIZE", "32");

    std::env::set_var("CLIENT_PREFIX", "client:");


    let redis_url = env::var("REDIS_URL").expect("REDIS_URL should be set");
    let client_prefix = env::var("CLIENT_PREFIX").unwrap_or("client:".parse().unwrap());

    // let mut rt = actix_rt::System::new("test");

    // Start, then open in browser, don't care about this finishing.
    // let _ = rt.block_on(start_browser());

    // let repo = DataSource::new(&redis_url,  &client_prefix, None).unwrap();
    // let repo = DataSource::new(vec!["redis://49.234.147.154:7001".to_string(),"redis://49.234.137.250:7001".to_string(),"redis://49.234.132.121:7001".to_string()], Some("idreamsky@123".to_string()),  client_prefix).unwrap();
    // let repo = DataSource::new(vec!["106.52.187.25:9042"],  "cassandra", "Brysj@1gsycl", "xapi", "apps").unwrap();
    let repo = DataSource::new(&redis_url, &client_prefix, None, vec!["106.52.187.25:9042"], "cassandra", "Brysj@1gsycl", "xapi", "apps").unwrap();
    // let repo = RedisClusterScyllaCluster::new(vec!["redis://49.234.147.154:7001"], &client_prefix, Some(""), vec!["106.52.187.25:9042"], "cassandra", "Brysj@1gsycl", "xapi", "apps").unwrap();

    let oauth_db_service =
        DBRegistrar::new(repo);

    // let sys = actix::System::new();
    let state = State::preconf_db_registrar(oauth_db_service).start();

    // Create the main server instance
    HttpServer::new(move || {
        App::new()
            .data(state.clone())
            .wrap(Logger::default())
            .service(
                web::resource("/authorize")
                    .route(web::get().to(get_authorize))
                    .route(web::post().to(post_authorize)),
            )
            .route("/token", web::post().to(token))
            .route("/refresh", web::post().to(refresh))
            .route("/", web::get().to(index))
    })
        .bind("localhost:8020")
        .expect("Failed to bind to socket")
        .run().await;

    support::dummy_client();
    // Run the rest of the system.
    // let _ = rt.run();
}

struct State {
    endpoint: Generic<
        DBRegistrar,
        AuthMap<RandomGenerator>,
        TokenMap<RandomGenerator>,
        Vacant,
        Vec<Scope>,
        fn() -> OAuthResponse,
    >,
}

impl State {
    pub fn preconf_db_registrar(db_service: DBRegistrar) -> Self {
        State {
            endpoint: Generic {
                // A redis db registrar, user can use regist() function to pre regist some clients.
                registrar: db_service,

                // Authorization tokens are 16 byte random keys to a memory hash map.
                authorizer: AuthMap::new(RandomGenerator::new(16)),
                // Bearer tokens are also random generated but 256-bit tokens, since they live longer
                // and this examples is somewhat paranoid.
                //
                // We could also use a `TokenSigner::ephemeral` here to create signed tokens which can
                // be read and parsed by anyone, but not maliciously created. However, they can not be
                // revoked and thus don't offer even longer lived refresh tokens.
                issuer: TokenMap::new(RandomGenerator::new(16)),

                solicitor: Vacant,

                // A single scope that will guard resources for this endpoint
                scopes: vec!["default-scope".parse().unwrap()],

                response: OAuthResponse::ok,
            },
        }
    }

    pub fn with_solicitor<'a, S>(
        &'a mut self, solicitor: S,
    ) -> impl Endpoint<OAuthRequest, Error=WebError> + 'a
        where
            S: OwnerSolicitor<OAuthRequest> + 'static,
    {
        debug!("with_solicitor");
        ErrorInto::new(Generic {
            authorizer: &mut self.endpoint.authorizer,
            registrar: &mut self.endpoint.registrar,
            issuer: &mut self.endpoint.issuer,
            solicitor,
            scopes: &mut self.endpoint.scopes,
            response: OAuthResponse::ok,
        })
    }
}


impl Actor for State {
    type Context = Context<Self>;
}

impl<Op> Handler<OAuthMessage<Op, Extras>> for State
    where
        Op: OAuthOperation,
{
    type Result = Result<Op::Item, Op::Error>;

    fn handle(&mut self, msg: OAuthMessage<Op, Extras>, _: &mut Self::Context) -> Self::Result {
        debug!("State handle OAuthMessage");
        let (op, ex) = msg.into_inner();

        match ex {
            Extras::AuthGet => {
                let solicitor = FnSolicitor(move |_: &mut OAuthRequest, pre_grant: Solicitation| {
                    // This will display a page to the user asking for his permission to proceed. The submitted form
                    // will then trigger the other authorization handler which actually completes the flow.
                    OwnerConsent::InProgress(
                        OAuthResponse::ok()
                            .content_type("text/html")
                            .unwrap()
                            .body(&crate::support::consent_page_html("/authorize".into(), pre_grant)),
                    )
                });

                op.run(self.with_solicitor(solicitor))
            }
            Extras::AuthPost(query_string) => {
                let solicitor = FnSolicitor(move |_: &mut OAuthRequest, _: Solicitation| {
                    if query_string.contains("allow") {
                        OwnerConsent::Authorized("dummy user".to_owned())
                    } else {
                        OwnerConsent::Denied
                    }
                });

                op.run(self.with_solicitor(solicitor))
            }
            _ => op.run(&mut self.endpoint),
        }
    }
}


#[tokio::test]
async fn test_refresh() {
    let refresh_token = "aoSBc8n1J97TFBSuDmFSxQ==";
    let mut headers = HashMap::new();
    // headers.insert("authorization".to_string(), format!("Basic {}", access_token));

    #[derive(Serialize)]
    struct Request {
        grant_type: String,
        refresh_token: String,
    }
    let body = Request { grant_type: "refresh_token".to_string(), refresh_token: refresh_token.to_string() };
    let body = serde_json::to_string(&body).unwrap();
    let res = post_json_timeout("http://127.0.0.1:8020",
                                &format!("/refresh?grant_type=refresh_token&refresh_token={}", refresh_token), "", Some(headers), 20).await.unwrap_or_default();
    println!("{}", res);
}

pub async fn post_json_timeout(host: &str, path: &str, content: &str, headers: Option<HashMap<String, String>>, timeout: u64) -> anyhow::Result<String> {
    let req_url = format!("{}{}", host, path);

    let cli = reqwest::Client::builder().danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(timeout)).build().unwrap();

    let mut req = cli.post(&req_url).header("Content-Type", "application/json");
    // let mut req = cli.post(&req_url).header("Content-Type", "text/plain");
    if let Some(h) = headers {
        for (k, v) in &h {
            req = req.header(k, v);
        }
    }

    let req_builder = req.body(content.to_string());
    // println!("edwin 52 {:?}", req_builder);
    let mut res = req_builder.send().unwrap();
    let body = res.text().unwrap();

    // println!("edwin 56 {}", body);

    Ok(body)
}
