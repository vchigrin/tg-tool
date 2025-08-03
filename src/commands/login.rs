use eyre::Result;
use grammers_client::{session::Session, Client, Config, SignInError};
use std::fs;
use std::io;
use std::io::BufRead;
use std::path;

const API_ID: i32 = match i32::from_str_radix(env!("TG_ID"), 10) {
    Ok(v) => v,
    Err(_) => {
        panic!("Invalid TG_ID environment variable")
    }
};
const API_HASH: &str = env!("TG_HASH");

fn prompt(message: &str) -> Result<String> {
    println!("{message}");
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut line = String::new();
    stdin.read_line(&mut line)?;
    Ok(line)
}

pub async fn handle_login_command(session_file: &path::Path) -> Result<()> {
    let session = Session::new();
    let client = Client::connect(Config {
        session,
        api_id: API_ID,
        api_hash: API_HASH.to_string(),
        params: Default::default(),
    })
    .await?;
    if !client.is_authorized().await? {
        let phone = prompt("Enter your phone number (international format): ")?;
        let token = client.request_login_code(&phone).await?;
        let code = prompt("Enter the code you received: ")?;
        let signed_in = client.sign_in(&token, &code).await;
        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                // Note: this `prompt` method will echo the password in the console.
                //       Real code might want to use a better way to handle this.
                let hint = password_token.hint().unwrap_or("None");
                let prompt_message = format!("Enter the password (hint {}): ", &hint);
                let password = rpassword::prompt_password(prompt_message).unwrap();

                client
                    .check_password(password_token, password.trim())
                    .await?;
            }
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        }
    }
    fs::File::create(session_file)?;
    client.session().save_to_file(session_file)?;
    Ok(())
}

pub async fn make_client_from_session_file(session_file: &path::Path) -> Result<Client> {
    let session = Session::load_file(session_file)?;
    let client = Client::connect(Config {
        session,
        api_id: API_ID,
        api_hash: API_HASH.to_string(),
        params: Default::default(),
    })
    .await?;
    Ok(client)
}

pub async fn handle_logout_command(session_file: &path::Path) -> Result<()> {
    let client = make_client_from_session_file(session_file).await?;
    client.sign_out().await?;
    fs::remove_file(session_file)?;
    Ok(())
}
