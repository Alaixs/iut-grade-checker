use reqwest::{Response, Client};
use select::document::Document;
use select::predicate::Name;
use std::env;
use dotenv::dotenv;
use serde_json::Value;
use anyhow::{Result, Context};
use std::convert::TryFrom;
use std::collections::HashMap;


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

    return Ok(());
}



async fn get_ues(client: &Client) -> Result<Vec<f32>> {

    let mut semestres_to_fetch: Vec<u64> = vec![];
    let mut grades: Vec<f32> = vec![];

    let r: Response = client.get("https://notes.iut-larochelle.fr/services/data.php?q=dataPremièreConnexion")
        .send()
        .await?;
    r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

    let json_response: Value = r.json().await?;

    for semestre in json_response["semestres"].as_array().context("Erreur lors de la récupération des semestres")?
    {
        // Si le semestre n'est pas le semestre de dataPremièreConnexion, on l'ajoute à la liste des semestres à récupérer
        if semestre["formsemestre_id"] != json_response["relevé"]["formsemestre_id"] 
        {
            semestres_to_fetch.push(semestre["formsemestre_id"].as_u64().context("Erreur lors de la récupération des ids des semestres")?);
        }
        
    }

    for ue in json_response["relevé"]["ues"].as_object().context("Erreur lors de la récupération des ues")?
    {
        // Ces trous de balle ont mis la moyenne en string, donc on récupère le str puis on parse en float
        grades.push(ue.1["moyenne"]["value"].as_str().context("Erreur lors de la récupération des moyennes")?.parse::<f32>().context("Erreur lors de la récupération des moyennes")?);
    }

    for id in &semestres_to_fetch
    {

        let r: Response = client.get("https://notes.iut-larochelle.fr/services/data.php?q=relevéEtudiant&semestre=".to_owned()+id.to_string().as_str())
            .send()
            .await?;
        r.error_for_status_ref().context("Erreur lors de la récupération des données")?;


        let json_response: Value = r.json().await?;
       
        for ue in json_response["relevé"]["ues"].as_object().context("Erreur lors de la récupération des ues")?
        {
            let grade_index: usize =  usize::try_from(ue.0.chars().last().context("Erreur lors de la récupération de l'ue")?.to_digit(10).context("Erreur lors de la récupération de l'ue")?).context("Erreur lors de la récupération de l'ue")?;
            let grade: &str = ue.1["moyenne"]["value"].as_str().context("Erreur lors de la récupération des moyennes")?;
            let value_to_add: f32;
            if grade.parse::<f32>().is_ok()
            {
                value_to_add = grade.parse::<f32>().context("Erreur lors de la récupération des moyennes")?;
            } else {
                value_to_add = grades[grade_index - 1];
            }
            grades[grade_index - 1] += value_to_add;
        }
    }
    grades = grades.iter().map(|x| x/f32::from(i16::try_from(semestres_to_fetch.len()+1).unwrap())).collect();

    return Ok(grades);
    
}

async fn get_saes(client: &Client) -> Result<HashMap<String, String>> {
    let r: Response = client
        .get("https://notes.iut-larochelle.fr/services/data.php?q=dataPremièreConnexion")
        .send()
        .await?;
    r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

    let json_response: Value = r.json().await?;
    let mut semestres_to_fetch: Vec<u64> = vec![];

    for semestre in json_response["semestres"]
        .as_array()
        .context("Erreur lors de la récupération des semestres")?
    {
        if semestre["formsemestre_id"] != json_response["relevé"]["formsemestre_id"] {
            semestres_to_fetch.push(
                semestre["formsemestre_id"]
                    .as_u64()
                    .context("Erreur lors de la récupération des ids des semestres")?,
            );
        }
    }

    let mut saes_dict: HashMap<String, String> = HashMap::new();

    for sae in json_response["relevé"]["saes"]
        .as_object()
        .context("Erreur lors de la récupération des ues")?
    {
        let sae_name = sae.0.to_owned();
        for evaluation in sae.1["evaluations"]
            .as_array()
            .context("Erreur lors de la récupération des ues")?
        {
            let note_value = evaluation["note"]["value"]
                .as_str()
                .unwrap()
                .to_owned();
            saes_dict.insert(sae_name.clone(), note_value);
        }
    }

    Ok(saes_dict)
}


async fn get_ressources(client: &Client) -> Result<HashMap<String, String>> {
    let r: Response = client
        .get("https://notes.iut-larochelle.fr/services/data.php?q=dataPremièreConnexion")
        .send()
        .await?;
    r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

    let json_response: Value = r.json().await?;
    let mut semestres_to_fetch: Vec<u64> = vec![];

    for semestre in json_response["semestres"]
        .as_array()
        .context("Erreur lors de la récupération des semestres")?
    {
        if semestre["formsemestre_id"] != json_response["relevé"]["formsemestre_id"] {
            semestres_to_fetch.push(
                semestre["formsemestre_id"]
                    .as_u64()
                    .context("Erreur lors de la récupération des ids des semestres")?,
            );
        }
    }

    let mut ressources_dict: HashMap<String, String> = HashMap::new();

    for ressource in json_response["relevé"]["ressources"]
        .as_object()
        .context("Erreur lors de la récupération des ues")?
    {
        let ressource_name = ressource.0.to_owned();
        for evaluation in ressource.1["evaluations"]
            .as_array()
            .context("Erreur lors de la récupération des ues")?
        {
            let note_value = evaluation["note"]["value"]
                .as_str()
                .unwrap()
                .to_owned();
            ressources_dict.insert(ressource_name.clone(), note_value);
        }
    }

    Ok(ressources_dict)
}


#[tokio::main]
async fn main() -> Result<()> {
    // Charger les variables d'environnement à partir du fichier .env
    dotenv().ok();

    // Récupérer le nom d'utilisateur et le mot de passe à partir des variables d'environnement

    let client: Client = Client::builder().cookie_store(true).build()?;


    get_cookies(&client).await?;
    // Effectuer une requête GET pour récupérer la page de connexion

    let grades: Vec<f32> = get_ues(&client).await?;

    for grade in &grades
    {
        println!("{}", grade);
    }


    for saes in get_saes(&client).await?.iter()
    {
        println!("{} : {}", saes.0, saes.1);
    }

    for ressources in get_ressources(&client).await?.iter()
    {
        println!("{} : {}", ressources.0, ressources.1);
    }





    return Ok(());
}
