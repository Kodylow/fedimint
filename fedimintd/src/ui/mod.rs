use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use askama::Template;
use axum::extract::{Extension, Form};
use axum::response::Redirect;
use axum::{
    routing::{get, post},
    Router,
};
use fedimint_api::config::BitcoindRpcCfg;
use fedimint_core::config::ClientConfig;
use fedimint_server::config::ServerConfig;
use http::StatusCode;
use mint_client::api::WsFederationConnect;
use qrcode_generator::QrCodeEcc;
use rand::rngs::OsRng;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;

use crate::ui::distributedgen;

fn run_fedimint(state: &mut RwLockWriteGuard<State>) {
    let sender = state.sender.clone();
    tokio::task::spawn(async move {
        // Tell fedimintd that setup is complete
        sender
            .send(UiMessage::SetupComplete)
            .await
            .expect("failed to send over channel");
    });
    state.running = true;
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Guardian {
    name: String,
    connection_string: String,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    federation_name: String,
    running: bool,
    federation_connection_string: String,
}

async fn home(Extension(state): Extension<MutableState>) -> HomeTemplate {
    let state = state.read().unwrap();
    let federation_connection_string = match state.client_config.clone() {
        Some(client_config) => {
            let connect_info = WsFederationConnect::from(&client_config);
            serde_json::to_string(&connect_info).unwrap()
        }
        None => "".into(),
    };

    HomeTemplate {
        federation_name: state.federation_name.clone(),
        running: state.running,
        federation_connection_string,
    }
}

#[derive(Template)]
#[template(path = "dealer.html")]
struct DealerTemplate {
    count_guardians: Vec<usize>,
    guardian: Guardian,
}

async fn dealer(Extension(state): Extension<MutableState>) -> DealerTemplate {
    let state = state.read().unwrap();
    DealerTemplate {
        guardian: state.guardians[0].clone(),
        count_guardians: state.count_guardians.clone(),
    }
}

async fn add_guardian(
    Extension(state): Extension<MutableState>,
    Form(form): Form<Guardian>,
) -> Result<Redirect, (StatusCode, String)> {
    state.write().unwrap().guardians.push(Guardian {
        connection_string: form.connection_string,
        name: form.name,
    });
    Ok(Redirect::to("/dealer".parse().unwrap()))
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct FedName {
    federation_name: String,
    btc_rpc: String,
}

async fn deal(
    Extension(state): Extension<MutableState>,
    Form(form): Form<FedName>,
) -> Result<Redirect, (StatusCode, String)> {
    let mut state = state.write().unwrap();
    state.federation_name = form.federation_name;
    state.btc_rpc = Some(form.btc_rpc.clone());

    let parts: Vec<&str> = form.btc_rpc.split('@').collect();
    let user_pass = parts[0].to_string();
    let btc_rpc_address = parts[1].to_string();
    let parts: Vec<&str> = user_pass.split(':').collect();
    let btc_rpc_user = parts[0].to_string();
    let btc_rpc_pass = parts[1].to_string();
    let btc_rpc = BitcoindRpcCfg {
        btc_rpc_address,
        btc_rpc_user,
        btc_rpc_pass,
    };

    let (server_configs, client_config) = configgen(
        state.federation_name.clone(),
        state.guardians.clone(),
        btc_rpc,
    );
    state.server_configs = Some(server_configs.clone());
    state.client_config = Some(client_config.clone());

    tracing::info!("Generated configs");

    save_configs(&server_configs[0].1, &client_config, &state.cfg_path);
    run_fedimint(&mut state);

    Ok(Redirect::to("/configs".parse().unwrap()))
}

#[derive(Template)]
#[template(path = "url_connection.html")]
struct UrlConnection {}

async fn url_connection(Extension(_state): Extension<MutableState>) -> UrlConnection {
    UrlConnection {}
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct UrlForm {
    name: String,
    ipaddr: String,
    count_guardians: usize,
}

async fn set_url_connection(
    Extension(state): Extension<MutableState>,
    Form(form): Form<UrlForm>,
) -> Result<Redirect, (StatusCode, String)> {
    let mut state = state.write().unwrap();

    let tls_string = distributed_gen::gen_tls(&state.cfg_path, &form.ipaddr, 4000, &form.name);

    // update state
    state.connection_string = tls_string;
    state.count_guardians = (0..form.count_guardians).collect();
    state.guardians[0].name = form.name;
    state.guardians[0].connection_string = state.connection_string.clone();
    Ok(Redirect::to("/dealer".parse().unwrap()))
}

#[derive(Template)]
#[template(path = "player.html")]
struct PlayerTemplate {
    connection_string: String,
}

async fn player(Extension(state): Extension<MutableState>) -> PlayerTemplate {
    PlayerTemplate {
        connection_string: state.read().unwrap().connection_string.clone(),
    }
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ReceiveConfigsForm {
    server_config: String,
    client_config: String,
}

async fn receive_configs(
    Extension(state): Extension<MutableState>,
    Form(form): Form<ReceiveConfigsForm>,
) -> Result<Redirect, (StatusCode, String)> {
    let mut state = state.write().unwrap();
    let server_config: ServerConfig = serde_json::from_str(&form.server_config).unwrap();
    let client_config: ClientConfig = serde_json::from_str(&form.client_config).unwrap();
    save_configs(&server_config, &client_config, &state.cfg_path);

    // update state
    state.client_config = Some(client_config);
    state.federation_name = server_config.federation_name;

    // run fedimint
    run_fedimint(&mut state);

    Ok(Redirect::to("/".parse().unwrap()))
}

fn save_configs(server_config: &ServerConfig, client_config: &ClientConfig, cfg_path: &PathBuf) {
    // Recursively create config directory if it doesn't exist
    let parent = cfg_path.parent().unwrap();
    std::fs::create_dir_all(&parent).expect("Failed to create config directory");

    // Save the configs
    let cfg_file = std::fs::File::create(&cfg_path).expect("Could not create cfg file");
    serde_json::to_writer_pretty(cfg_file, &server_config).unwrap();
    let client_cfg_path = parent.join("client.json");
    let client_cfg_file =
        std::fs::File::create(&client_cfg_path).expect("Could not create cfg file");
    serde_json::to_writer_pretty(client_cfg_file, &client_config).unwrap();
}

#[derive(Template)]
#[template(path = "configs.html")]
struct DisplayConfigsTemplate {
    federation_name: String,
    server_configs: Vec<(Guardian, String)>,
    client_config: String,
    federation_connection_string: String,
}

async fn display_configs(Extension(state): Extension<MutableState>) -> DisplayConfigsTemplate {
    let state = state.read().unwrap();
    let server_configs = state
        .server_configs
        .clone()
        .unwrap()
        .into_iter()
        .map(|(guardian, cfg)| (guardian, serde_json::to_string(&cfg).unwrap()))
        .collect();
    let federation_connection_string = match state.client_config.clone() {
        Some(client_config) => {
            let connect_info = WsFederationConnect::from(&client_config);
            serde_json::to_string(&connect_info).unwrap()
        }
        None => "".into(),
    };
    DisplayConfigsTemplate {
        federation_name: state.federation_name.clone(),
        server_configs,
        client_config: serde_json::to_string(&state.client_config.as_ref().unwrap()).unwrap(),
        federation_connection_string,
    }
}

async fn qr(Extension(state): Extension<MutableState>) -> impl axum::response::IntoResponse {
    let client_config = state.read().unwrap().client_config.clone().unwrap();
    let connect_info = WsFederationConnect::from(&client_config);
    let string = serde_json::to_string(&connect_info).unwrap();
    let png_bytes: Vec<u8> = qrcode_generator::to_png_to_vec(string, QrCodeEcc::Low, 1024).unwrap();
    (
        axum::response::Headers([(axum::http::header::CONTENT_TYPE, "image/png")]),
        png_bytes,
    )
}

#[derive(Debug)]
struct State {
    federation_name: String,
    count_guardians: Vec<usize>,
    guardians: Vec<Guardian>,
    running: bool,
    cfg_path: PathBuf,
    connection_string: String,
    sender: Sender<UiMessage>,
    server_configs: Option<Vec<(Guardian, ServerConfig)>>,
    client_config: Option<ClientConfig>,
    btc_rpc: Option<String>,
}
type MutableState = Arc<RwLock<State>>;

#[derive(Debug)]
pub enum UiMessage {
    SetupComplete,
}

pub async fn run_ui(cfg_path: PathBuf, sender: Sender<UiMessage>, port: u32) {
    let mut rng = OsRng;
    let secp = bitcoin::secp256k1::Secp256k1::new();

    let (_, pubkey) = secp.generate_keypair(&mut rng);
    let connection_string = format!("{}", pubkey);
    let guardians = vec![Guardian {
        connection_string: connection_string.clone(),
        name: "You".into(),
    }];

    // Default federation name
    let federation_name = "Cypherpunk".into();

    let state = Arc::new(RwLock::new(State {
        federation_name,
        count_guardians: vec![],
        guardians,
        running: false,
        cfg_path,
        connection_string,
        sender,
        server_configs: None,
        client_config: None,
        btc_rpc: None,
    }));

    let app = Router::new()
        .route("/", get(home))
        .route(
            "/url_connection",
            get(url_connection).post(set_url_connection),
        )
        .route("/player", get(player).post(receive_configs))
        .route("/dealer", get(dealer).post(add_guardian))
        .route("/configs", get(display_configs))
        .route("/deal", post(deal))
        .route("/qr", get(qr))
        .layer(Extension(state));

    let bind_addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    axum::Server::bind(&bind_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
