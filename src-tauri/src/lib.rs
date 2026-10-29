mod appearance;
mod auth;
mod catalog;
pub mod cli;
mod cookies;
mod dialogs;
mod docs;
mod encoding;
mod environment;
mod error;
mod export;
mod files;
mod flow;
mod grpc;
mod history;
mod import;
mod legacy;
mod mcp;
mod mock;
mod preferences;
mod request;
mod run_data;
mod script;
mod sensitive;
mod soap;
mod storage;
mod team;
mod text;
mod trash;
mod update;
mod websocket;

use appearance::fonts::FontStore;
use appearance::settings::{restore_window, theme_script};
use environment::vault::{
    CredentialStoreKey, DeviceCredential, Vault, install_platform_credential_store,
};
use flow::commands::FlowRuns;
use grpc::commands::GrpcDescriptors;
use mcp::manager::McpManager;
use mock::commands::MockServers;
use request::bodies::ResponseBodies;
use request::client::HttpClient;
use request::streams::EventStreams;
use storage::db::Database;
use tauri::Manager;
use team::access::DeviceKey;
use team::sync::TeamState;
use update::commands::PendingUpdate;
use websocket::manager::WsManager;

pub(crate) const DATABASE_FILE: &str = "codestamp.db";
const FONTS_DIR: &str = "fonts";
const REPOS_DIR: &str = "repos";
const MAIN_WINDOW: &str = "main";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_client = HttpClient::new().expect("failed to initialise the HTTP client");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(http_client)
        .manage(Vault::new(Box::new(CredentialStoreKey)))
        .manage(WsManager::default())
        .manage(McpManager::default())
        .manage(ResponseBodies::default())
        .manage(MockServers::default())
        .manage(EventStreams::default())
        .manage(GrpcDescriptors::default())
        .manage(FlowRuns::default())
        .manage(PendingUpdate::default())
        .setup(|app| {
            install_platform_credential_store()?;
            let data_dir = app.path().app_data_dir()?;
            legacy::adopt_folder(&data_dir)?;
            legacy::adopt_database(&data_dir, DATABASE_FILE)?;
            // This folder only holds the webview's cache, so a fresh one is fine when the old one
            // cannot move.
            let _ = legacy::adopt_folder(&app.path().app_local_data_dir()?);
            std::fs::create_dir_all(&data_dir)?;
            let database = Database::open(&data_dir.join(DATABASE_FILE))?;
            open_main_window(app, &database)?;
            let client = app.state::<HttpClient>();
            let vault = app.state::<Vault>();
            database.with(|conn| cookies::repo::restore(conn, &vault, client.jar()))?;
            // Saved settings that cannot be used (an unreadable key, say) leave the defaults in
            // place; Settings shows the reason so the user can fix them.
            if preferences::network::apply(&database, &vault, &client).is_err() {
                client.configure(&request::network::NetworkConfig::default())?;
            }
            let repos_dir = data_dir.join(REPOS_DIR);
            std::fs::create_dir_all(&repos_dir)?;
            let team = TeamState::new(repos_dir, DeviceKey::new(Box::new(DeviceCredential)));
            team.cleanup(&database)?;
            app.manage(database);
            app.manage(team);
            app.manage(FontStore::new(data_dir.join(FONTS_DIR)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            request::commands::send_http_request,
            request::commands::resolve_request_for_code,
            request::commands::graphql_schema,
            request::commands::stop_event_stream,
            grpc::commands::grpc_describe,
            grpc::commands::grpc_invoke,
            grpc::commands::pick_proto_file,
            flow::commands::list_flows,
            flow::commands::create_flow,
            flow::commands::get_flow,
            flow::commands::save_flow,
            flow::commands::delete_flow,
            flow::commands::run_flow,
            flow::commands::stop_flow,
            catalog::commands::catalog_overview,
            catalog::commands::catalog_detail,
            catalog::commands::save_catalog_info,
            catalog::commands::record_collection_run,
            docs::commands::collection_docs,
            docs::commands::save_description,
            docs::commands::search_docs,
            export::commands::export_docs,
            request::commands::pick_upload_file,
            cookies::commands::list_cookies,
            cookies::commands::reveal_cookie,
            cookies::commands::delete_cookie,
            cookies::commands::clear_cookies,
            storage::commands::duplicate_request,
            storage::commands::duplicate_section,
            storage::commands::duplicate_collection,
            export::commands::export_collection,
            export::commands::export_environment,
            export::commands::save_response_body,
            run_data::commands::pick_run_data,
            auth::commands::oauth_get_token,
            auth::commands::oauth_token_status,
            auth::commands::oauth_clear_token,
            storage::commands::list_collections,
            storage::commands::create_collection,
            storage::commands::rename_collection,
            storage::commands::set_collection_icon,
            storage::commands::collection_icons,
            storage::commands::suggest_collection_icon,
            storage::commands::delete_collection,
            storage::commands::get_collection,
            storage::commands::search_index,
            storage::commands::list_examples,
            storage::commands::get_example,
            storage::commands::save_example,
            storage::commands::rename_example,
            storage::commands::delete_example,
            mock::commands::mock_start,
            mock::commands::mock_stop,
            mock::commands::mock_status,
            storage::commands::create_section,
            storage::commands::rename_section,
            storage::commands::delete_section,
            storage::commands::create_request,
            storage::commands::get_request,
            storage::commands::update_request,
            storage::commands::delete_request,
            storage::commands::reorder_collection,
            storage::commands::set_collection_auth,
            trash::commands::list_deleted,
            trash::commands::restore_deleted,
            trash::commands::forget_deleted,
            trash::commands::forget_all_deleted,
            history::commands::list_history,
            history::commands::delete_history_entry,
            history::commands::clear_history,
            history::commands::save_history_to_collection,
            environment::commands::list_environments,
            environment::commands::create_environment,
            environment::commands::rename_environment,
            environment::commands::delete_environment,
            environment::commands::set_environment_collection,
            environment::commands::set_active_environment,
            environment::commands::get_environment_variables,
            environment::commands::save_environment_variables,
            environment::commands::reveal_secret,
            environment::commands::get_collection_variable_set,
            environment::commands::variable_hints,
            environment::commands::preview_url,
            import::commands::preview_import,
            import::commands::run_import,
            import::commands::replace_with_import,
            import::commands::parse_curl,
            import::commands::fetch_import,
            import::commands::preview_wsdl_import,
            import::commands::run_wsdl_import,
            soap::commands::pick_wsdl_file,
            soap::commands::soap_load_wsdl,
            websocket::commands::ws_connect,
            websocket::commands::ws_send,
            websocket::commands::ws_close,
            websocket::commands::ws_close_all,
            mcp::commands::mcp_connect,
            mcp::commands::mcp_request,
            mcp::commands::mcp_close,
            mcp::commands::mcp_close_all,
            appearance::commands::get_appearance,
            appearance::commands::save_appearance,
            appearance::commands::import_font,
            appearance::commands::read_font,
            appearance::commands::delete_font,
            preferences::commands::get_preferences,
            preferences::commands::save_preferences,
            preferences::commands::get_proxy,
            preferences::commands::save_proxy,
            preferences::commands::network_problem,
            preferences::commands::list_certificates,
            preferences::commands::add_ca_certificate,
            preferences::commands::add_client_certificate,
            preferences::commands::delete_certificate,
            preferences::commands::backup_data,
            preferences::commands::restore_data,
            preferences::commands::about_app,
            update::commands::check_for_update,
            update::commands::install_update,
            export::commands::save_capture,
            export::commands::save_report,
            team::commands::team_setup,
            team::commands::set_team_account,
            team::commands::list_git_accounts,
            team::commands::add_git_account,
            team::commands::remove_git_account,
            team::commands::check_git_account,
            team::commands::sign_in_git_account,
            team::commands::sign_in_team_collection,
            team::commands::sign_out_git_account,
            team::commands::git_changes_overview,
            team::commands::git_commits,
            team::commands::git_commit_detail,
            team::commands::git_request_activity,
            team::commands::list_team_links,
            team::commands::share_collection,
            team::commands::preview_share,
            team::commands::join_team_collection,
            team::commands::sync_team_collection,
            team::commands::access_overview,
            team::commands::create_access_group,
            team::commands::request_group_access,
            team::commands::grant_group_access,
            team::commands::decline_group_access,
            team::commands::remove_group_member,
            team::commands::repair_access_group,
            team::commands::unlink_team_collection,
            team::commands::request_versions,
            team::commands::request_version_changes,
            team::commands::restore_request_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Codestamp application");
}

/// The window is opened here rather than by Tauri at start-up (`create: false`), so the data
/// folders from before the rename have moved before the webview opens its storage in them.
/// The window opens centred and hidden, and shows once it has the saved theme's colours, so the
/// launch screen appears without a white flash.
fn open_main_window(
    app: &tauri::App,
    database: &Database,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW)
        .cloned();
    let Some(config) = config else {
        return Ok(());
    };
    let appearance = database.with(appearance::settings::current)?;
    let window = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?
        .center()
        .visible(false)
        .initialization_script(theme_script(&appearance)?)
        .build()?;
    restore_window(&window, &appearance)?;
    window.show()?;
    Ok(())
}
