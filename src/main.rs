use reqwest::{Response, Client};
use select::document::Document;
use select::predicate::Name;
use std::env;
use dotenv::dotenv;
use serde_json::Value;
use anyhow::{Result, Context};

const LOGIN_URL: &str = "https://authentification.univ-lr.fr/cas/login?service=https://notes.iut-larochelle.fr/services/doAuth.php?href=https://notes.iut-larochelle.fr/";


async fn get_cookies(client: &Client) -> Result<()> {
    let pre_auth: String = client.get(LOGIN_URL)
    .send()
    .await?
    .text()
    .await?;

    let username: String = env::var("USERNAME_ENT").context("Le nom d'utilisateur n'est pas défini dans le fichier .env")?;
    let password: String = env::var("PASSWORD_ENT").context("Le mot de passe n'est pas défini dans le fichier .env")?;

    // Utiliser la bibliothèque select pour extraire la valeur d'exécution
    let document: Document = Document::from(pre_auth.as_str());
    let exec_value: &str = document
        .find(Name("input"))
        .filter(|n| n.attr("name").unwrap_or("") == "execution")
        .next()
        .and_then(|n: select::node::Node<'_>| n.attr("value"))
        .context("Valeur d'exécution introuvable")?;

    // Données du formulaire

    let form_data: [(&str, &str); 5] = [
        ("username", username.as_str()),
        ("password", password.as_str()),
        ("execution", exec_value),
        ("_eventId", "submit"),
        ("geolocation", ""),
    ];

    // Effectuer une requête POST pour se connecter
    let auth: Response = client.post(LOGIN_URL)
        .form(&form_data)
        .send()
        .await?;
    auth.error_for_status_ref().context("Erreur lors de la connexion")?;

    Ok(())
}



async fn get_ues(client: &Client) -> Result<()> {
    Ok(())
}


#[tokio::main]
async fn main() -> Result<()> {
    // Charger les variables d'environnement à partir du fichier .env
    dotenv().ok();

    // Récupérer le nom d'utilisateur et le mot de passe à partir des variables d'environnement

    let client: Client = Client::builder().cookie_store(true).build()?;


    get_cookies(&client).await?;
    // Effectuer une requête GET pour récupérer la page de connexion
    

    // Effectuer une requête GET pour récupérer les données
    let r: Response = client.get("https://notes.iut-larochelle.fr/services/data.php?q=dataPremièreConnexion")
        .send()
        .await?;
    r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

    let text_response: String = r.text().await?;
    let json_response: Value = serde_json::from_str(&text_response)?;

    println!("{:?}", json_response);

    Ok(())
}
