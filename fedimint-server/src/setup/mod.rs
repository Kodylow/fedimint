mod configgen;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use askama::Template;
use axum::extract::{Extension, Form};
use axum::response::Redirect;
use axum::{
    routing::{get, post},
    Router,
};
use fedimint_core::config::ClientConfig;
use http::StatusCode;
use qrcode_generator::QrCodeEcc;
use rand::rngs::OsRng;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;

use crate::setup::configgen::configgen;
use crate::ServerConfig;
use mint_client::api::WsFederationConnect;

fn run_fedimint(state: &mut RwLockWriteGuard<State>) {
    let sender = state.sender.clone();
    tokio::task::spawn(async move {
        // trying to trigger the receive message
        sender.send(()).await.expect("failed to send over channel");
    }); // FIXME: it won't let me await this
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
    running: bool,
    can_run: bool,
}

async fn home(Extension(state): Extension<MutableState>) -> HomeTemplate {
    let state = state.read().unwrap();
    let can_run = Path::new(&state.cfg_path.clone()).is_file() && !state.running;
    HomeTemplate {
        running: state.running,
        can_run,
    }
}

#[derive(Template)]
#[template(path = "dealer.html")]
struct DealerTemplate {
    guardians: Vec<Guardian>,
}

async fn dealer(Extension(state): Extension<MutableState>) -> DealerTemplate {
    DealerTemplate {
        guardians: state.read().unwrap().guardians.clone(),
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

async fn deal(Extension(state): Extension<MutableState>) -> Result<Redirect, (StatusCode, String)> {
    let mut state = state.write().unwrap();
    let (server_configs, client_config) = configgen(state.guardians.clone());
    state.server_configs = Some(server_configs.clone());
    state.client_config = Some(client_config.clone());

    tracing::info!("Generated configs");

    // TODO: print these configs to the screen ...
    save_configs(&server_configs[0].1, &client_config, &state.cfg_path);
    run_fedimint(&mut state);

    Ok(Redirect::to("/configs".parse().unwrap()))
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

    // run fedimint
    run_fedimint(&mut state);

    Ok(Redirect::to("/".parse().unwrap()))
}

fn save_configs(server_config: &ServerConfig, client_config: &ClientConfig, cfg_path: &PathBuf) {
    // Recursively create config directory if it doesn't exist
    let parent = cfg_path.parent().unwrap();
    std::fs::create_dir_all(&parent).expect("Failed to create config directory");

    // Save the configs
    tracing::info!("{:?}", cfg_path);
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
    server_configs: Vec<(Guardian, String)>,
    client_config: String,
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
    DisplayConfigsTemplate {
        server_configs,
        client_config: serde_json::to_string(&state.client_config.as_ref().unwrap()).unwrap(),
    }
}

async fn qr(Extension(state): Extension<MutableState>) -> impl axum::response::IntoResponse {
    let client_config = state.read().unwrap().client_config.clone().unwrap();
    let connect_info = WsFederationConnect::from(&client_config);
    let string = serde_json::to_string(&connect_info).unwrap();
    // this was a hack to do a remote demo ... leaving just in case I need to do another!
    // .replace("127.0.0.1", "188.166.55.8");
    let png_bytes: Vec<u8> = qrcode_generator::to_png_to_vec(string, QrCodeEcc::Low, 1024).unwrap();
    (
        axum::response::Headers([(axum::http::header::CONTENT_TYPE, "image/png")]),
        png_bytes,
    )
}

// TODO: write cfg_path and db_path into state so we don't re-compute them
#[derive(Debug)]
struct State {
    guardians: Vec<Guardian>,
    // TODO: map name to peer id
    running: bool,
    cfg_path: PathBuf,
    connection_string: String,
    sender: Sender<()>,
    server_configs: Option<Vec<(Guardian, ServerConfig)>>,
    client_config: Option<ClientConfig>,
}
type MutableState = Arc<RwLock<State>>;

pub async fn run_ui_setup(cfg_path: PathBuf, port: u16, sender: Sender<()>) {
    let mut rng = OsRng::new().unwrap();
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let (_, pubkey) = secp.generate_keypair(&mut rng);
    let our_ip = "127.0.0.1"; // TODO: get our actual IP ... or pass in as argument???
    let connection_string = format!("{}@{}:{}", pubkey, our_ip, port);
    let guardians = vec![Guardian {
        connection_string: connection_string.clone(),
        name: "You".into(),
    }];

    let state = Arc::new(RwLock::new(State {
        guardians,
        running: false,
        cfg_path,
        connection_string,
        sender,
        server_configs: None,
        client_config: None,
    }));

    let app = Router::new()
        .route("/", get(home))
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
