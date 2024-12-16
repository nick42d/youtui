use crate::get_api;
use crate::get_config_dir;
use crate::Cli;
use crate::RuntimeInfo;
use crate::OAUTH_FILENAME;
use anyhow::Result;
use futures::future::try_join_all;
use querybuilder::command_to_query;
use querybuilder::CliQuery;
use querybuilder::QueryType;
use std::path::PathBuf;
use ytmapi_rs::{generate_oauth_code_and_url, generate_oauth_token};

mod querybuilder;

pub async fn handle_cli_command(cli: Cli, rt: RuntimeInfo) -> Result<()> {
    let config = rt.config;
    match cli {
        // TODO: Block this action using type system.
        Cli {
            command: None,
            show_source: true,
            ..
        } => println!("Show source requires an associated API command"),
        Cli {
            command: None,
            input_json: Some(_),
            ..
        } => println!("API command must be provided when providing an input json file"),
        Cli {
            command: None,
            input_json: None,
            show_source: false,
        } => println!("No command provided"),
        Cli {
            command: Some(command),
            input_json: Some(input_array),
            show_source,
        } => {
            let source_futures = input_array.into_iter().map(tokio::fs::read_to_string);
            let sources = try_join_all(source_futures).await?;
            let cli_query = CliQuery {
                query_type: QueryType::FromSourceFiles(sources),
                show_source,
            };
            let api = get_api(&config).await?;
            let res = command_to_query(command, cli_query, api).await?;
            println!("{res}");
        }
        Cli {
            command: Some(command),
            input_json: None,
            show_source,
        } => {
            let cli_query = CliQuery {
                query_type: QueryType::FromApi,
                show_source,
            };
            let api = get_api(&config).await?;
            let res = command_to_query(command, cli_query, api).await?;
            println!("{res}");
        }
    }
    Ok(())
}
pub async fn get_and_output_oauth_token(
    file_name: Option<PathBuf>,
    write_to_stdout: bool,
) -> Result<()> {
    let token_str = get_oauth_token().await?;
    match (file_name, write_to_stdout) {
        (Some(file_name), _) => {
            tokio::fs::write(&file_name, &token_str).await?;
            println!("Wrote Oauth token to {}", file_name.display());
        }
        (None, false) => {
            let mut path = get_config_dir()?;
            path.push(OAUTH_FILENAME);
            tokio::fs::write(&path, &token_str).await?;
            println!("Wrote Oauth token to {}", path.display());
        }
        (None, true) => (),
    };
    if write_to_stdout {
        println!("{token_str}");
    }
    Ok(())
}
async fn get_oauth_token() -> Result<String> {
    let client = ytmapi_rs::client::Client::new_rustls_tls()?;
    let (code, url) = generate_oauth_code_and_url(&client).await?;
    // Hack to wait for input
    println!("Go to {url}, finish the login flow, and press enter when done");
    let mut _buf = String::new();
    let _ = std::io::stdin().read_line(&mut _buf);
    let token = generate_oauth_token(&client, code).await?;
    Ok(serde_json::to_string_pretty(&token)?)
}
