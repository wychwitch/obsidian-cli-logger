use clap::Parser;
use preferences::{AppInfo, Preferences, PreferencesMap};
use reqwest;
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use urlencoding::{decode, encode};

const APP_INFO: AppInfo = AppInfo {
    name: "Obsidian cli quick-logger",
    author: "wych(witch) <wych@wychwit.ch>",
};

#[derive(clap::Parser)]
#[command(name = APP_INFO.name)]
#[command(author = APP_INFO.author)]
#[command(version = "0.1.0")]
#[command(about = "Quickly sends things to obsidian over cli", long_about = None, arg_required_else_help = true, after_help = "NOTE: Target is relative to root. Must not begin or end with a slash. Can accept periodic note specifications instead such as daily or quarterly.")]
struct Args {
    #[command(subcommand)]
    action: Action,
}

//an enum of all available commands
#[derive(clap::Subcommand, Debug)]
enum Action {
    /// Log to a specified file. USAGE: obs log <TEXT>
    ///
    /// Log argument, sends the string to the obsidian
    Log { body: String },
    /// set the api-key USAGE: obs key <API_KEY>
    Key { api_key: String },
    ///Change the target file. USAGE obs target <TARGET_FILE>
    Target { target_file: String },
    ///retrieves the currently set target file
    GetTarget,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let save_path = "./obsidian-log";
    let settings = PreferencesMap::<String>::load(&APP_INFO, save_path);

    //checking if the the settings exist, if not init
    let mut settings = match settings {
        Ok(set) => {
            let mut checked_set: HashMap<String, String> = HashMap::new();
            if set.contains_key("api_key") {
                checked_set.insert("api_key".to_string(), set["api_key"].to_string());
            }
            //TODO add support for multiple targets with ids
            if set.contains_key("target_file") {
                checked_set.insert("target_file".to_string(), set["target_file"].to_string());
            } else {
                checked_set.insert("target_file".to_string(), "/periodic/daily/".to_string());
            }
            checked_set
        }
        Err(_) => PreferencesMap::new(),
    };

    //Parsing the arguments passed to program
    let args = Args::parse();
    match args.action {
        Action::Log { body } => {
            if settings.contains_key("api_key") {
                send_log(
                    settings["api_key"].as_str(),
                    &body,
                    settings["target_file"].as_str(),
                )
                .await?;
            } else {
                println!("You need to set your api key with $ obs key <API_KEY>")
            }
        }
        Action::Key { api_key } => {
            settings.insert("api_key".to_string(), api_key);
            let save_result = settings.save(&APP_INFO, save_path);
            assert!(save_result.is_ok());
            println!("Key saved")
        }
        Action::Target { target_file } => {
            let target_file = match target_file.as_str() {
                "daily" => "/periodic/daily/".into(),
                "weekly" => "/periodic/weekly/".into(),
                "monthly" => "/periodic/monthly/".into(),
                "quarterly" => "/periodic/quarterly/".into(),
                "yearly" => "/periodic/yearly/".into(),
                _ => encode_target(&target_file),
            };
            settings.insert("target_file".to_string(), target_file.to_string());
            let save_result = settings.save(&APP_INFO, save_path);
            assert!(save_result.is_ok());
            let decoded = decode(&target_file).expect("UTF-8");
            println!("{decoded} set to target file")
        }
        Action::GetTarget => {
            println!("{}", decode(&settings["target_file"]).expect("UTF-8"))
        }
    }

    Ok(())
}

fn encode_target(target: &str) -> String {
    let split_target: Vec<String> = target
        .split("/")
        .map(|item_name| encode(item_name).into_owned())
        .collect();
    "/vault/".to_string() + split_target.join("/").as_str()
}

async fn send_log(authorization: &str, body: &str, target: &str) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    println!("{}", target);
    let res = client
        // only http is supported for now
        .post("http://127.0.0.1:27123".to_string() + target)
        .header("Accept", "*/*")
        .header("Authorization", "Bearer ".to_string() + authorization)
        .header("Content-Type", "text/markdown")
        .timeout(Duration::from_secs(3))
        .body(body.to_string())
        .send()
        .await?
        .text()
        .await?;
    //prints the result to stout
    //todo make this prettier??
    println!("{:}", res);
    Ok(())
}
