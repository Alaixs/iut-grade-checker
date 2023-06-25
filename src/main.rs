use anyhow::Ok;
use reqwest::{Response, Client};
use select::document::Document;
use select::predicate::Name;
use std::env;
use dotenv::dotenv;
use serde_json::Value;
use serde_json::json;
use anyhow::{Result, Context};
use std::convert::TryFrom;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;


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

    println!("{:?}", grades);

    return Ok(grades);
    
}

async fn get_saes(client: &Client) -> Result<HashMap<String, f32>> {
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

    let mut saes_dict: HashMap<String, f32> = HashMap::new();
    let mut note_final: f32 = 0.0;

    //boucle de récupération des saes
    for sae in json_response["relevé"]["saes"]
        .as_object()
        .context("Erreur lors de la récupération des ues")?
    {
        let sae_name = sae.0.to_owned();
        
        //boucle de récupération des évaluations de chaque sae
        for evaluation in sae.1["evaluations"]
            .as_array()
            .context("Erreur lors de la récupération des ues")?
        {
            let note_value = evaluation["note"]["value"]
                .as_str()
                .unwrap()
                .to_owned();
            if note_value == "~"
            {
                 note_final += 0.0;
            } else {
                note_final += note_value.parse::<f32>().unwrap();
            }
            saes_dict.insert(sae_name.clone(), note_final);
            //on renitialise la note final
            note_final = 0.0;
        }
    }

    Ok(saes_dict)
}


async fn get_ressources(client: &Client) -> Result<HashMap<String, f32>> {
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

    let mut ressources_dict: HashMap<String, f32> = HashMap::new();
    let mut note_final: f32 = 0.0;

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
            
            if note_value == "~"
            {
                 note_final += 0.0;
            } else {
                note_final += note_value.parse::<f32>().unwrap();
            }
        }
        ressources_dict.insert(ressource_name.clone(), note_final);
        //on renitialise la note final
        note_final = 0.0;
    }

    Ok(ressources_dict)
}

fn compare_dictionnaries(dict1: &HashMap<String, f32>, dict2: &HashMap<String, f32>) -> Vec<String>
{
    let mut differences: Vec<String> = vec![];

    for (key, value) in dict1
    {
        if dict2.contains_key(key)
        {
            if dict2[key] != *value
            {
                differences.push(key.to_owned());
            }
        } else {
            differences.push(key.to_owned());
        }
    }

    return differences;
}

async fn send_webhook(payload: Vec<String>, year_valid: bool) -> Result<()> {
    let client = Client::new();

    let webhook_url: String = env::var("WEBHOOK_URL").unwrap();

    let payload_str = payload.join("\n"); // Convertir le Vec<String> en une chaîne de caractères

    let mut json_payload = json!({
        "content": null,
        "embeds": [
            {
                "title": "Changement détecté",
                "description": format!("Une modification a été détectée sur {}", payload_str),
                "color": 5814783
            }
        ],
        "attachments": []
    });

    if !year_valid {
        json_payload["embeds"][0]["description"] = json!("Tu n'as pas ton année");
    } else {
        json_payload["embeds"][0]["description"] = json!("Tu as ton année");
    }

    client
        .post(&webhook_url)
        .json(&json_payload)
        .send()
        .await?;

    Ok(())
}

async fn get_is_year(ues: Vec<f32>) -> Result<bool> {
    let mut get_is_year : bool = true;
    let mut count_below_10 = 0;

    for ue_nb in ues {
        if ue_nb < 8.0 {
            get_is_year = false;
        }
        if ue_nb < 10.0 {
            count_below_10 += 1;
        }
        if count_below_10 >= 2 {
            get_is_year = false;
        }
    }
    Ok(get_is_year)
}




#[tokio::main]
async fn main() -> Result<()> {
    // Charger les variables d'environnement à partir du fichier .env
    dotenv().ok();

    // Récupérer le nom d'utilisateur et le mot de passe à partir des variables d'environnement

    let client: Client = Client::builder().cookie_store(true).build()?;


    get_cookies(&client).await?;
    // Effectuer une requête GET pour récupérer la page de connexion


    let mut old_vec_saes: HashMap<String, f32> = get_saes(&client).await?;
    let mut old_vec_ressources: HashMap<String, f32> = get_ressources(&client).await?;
    let mut a_vec: Vec<String>;
    let mut new_vec_ressources: HashMap<String, f32>;
    let mut new_vec_saes: HashMap<String, f32>;

    loop
    {
        println!("refresh...");
        new_vec_saes= get_saes(&client).await?;
        new_vec_ressources= get_ressources(&client).await?;

        a_vec = compare_dictionnaries(&old_vec_saes, &new_vec_saes);
        if !a_vec.is_empty()
        {
            println!("{:?}", a_vec);

            //on envoie le webhook
            send_webhook(a_vec, get_is_year(get_ues(&client).await?).await?).await?;

            //on change l'ancien dictionnaire par le nouveau
            old_vec_saes = new_vec_saes;
        }
        a_vec = compare_dictionnaries(&old_vec_ressources, &new_vec_ressources);
        if !a_vec.is_empty()
        {
            println!("{:?}", a_vec);

            //on envoie le webhook
            send_webhook(a_vec, get_is_year(get_ues(&client).await?).await?).await?;
            
            //on change l'ancien dictionnaire par le nouveau
            old_vec_ressources = new_vec_ressources;
        }
        thread::sleep(Duration::from_secs(300));
    }

}
