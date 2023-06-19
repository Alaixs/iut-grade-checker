use reqwest;
use select::document::Document;
use select::predicate::Name;
use std::env;
use dotenv::dotenv;
use serde_json::Value;
use anyhow::{Result, Context};

#[tokio::main]
async fn main() -> Result<()> {
    // Charger les variables d'environnement à partir du fichier .env
    dotenv().ok();

    // Récupérer le nom d'utilisateur et le mot de passe à partir des variables d'environnement
    let username = "xxx";
    let password = "xxx";

    let login_url = "https://authentification.univ-lr.fr/cas/login?service=https://notes.iut-larochelle.fr/services/doAuth.php?href=https://notes.iut-larochelle.fr/";

    let client = reqwest::Client::builder().cookie_store(true).build()?;

    // Effectuer une requête GET pour récupérer la page de connexion
    let pre_auth = client.get(login_url)
    .send()
    .await?
    .text()
    .await?;

    // Utiliser la bibliothèque select pour extraire la valeur d'exécution
    let document = Document::from(pre_auth.as_str());
    let exec_value = document
        .find(Name("input"))
        .filter(|n| n.attr("name").unwrap_or("") == "execution")
        .next()
        .and_then(|n| n.attr("value"))
        .context("Valeur d'exécution introuvable")?;

    // Données du formulaire
    let form_data = [
        ("username", username.to_string()),
        ("password", password.to_string()),
        ("execution", exec_value.to_string()),
        ("_eventId", "submit".to_string()),
        ("geolocation", "".to_string()),
    ];

    // Effectuer une requête POST pour se connecter
    let auth = client.post(login_url)
        .form(&form_data)
        .send()
        .await?;
    auth.error_for_status_ref().context("Erreur lors de la connexion")?;

    // Effectuer une requête GET pour récupérer les données
    let r = client.get("https://notes.iut-larochelle.fr/services/data.php?q=dataPremièreConnexion")
        .send()
        .await?;
    r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

    let text_response = r.text().await?;
    let json_response: Value = serde_json::from_str(&text_response)?;

    println!("{:?}", json_response);

    Ok(())
}
