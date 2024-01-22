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
use chrono;


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



async fn get_ues_and_data(client: &Client, map_saes: &mut HashMap<String, f32>, map_ressources: &mut HashMap<String, f32>) -> Result<Vec<f32>> {

// On récupère les données de dataPremièreConnexion et on récupère les ids des semestres restants
    let mut semestres_to_fetch: Vec<u64> = vec![];
    let mut ues_grades: Vec<f32> = vec![];

    let r: Response = client.get("https://notes.iut-larochelle.fr/services/data.php?q=dataPremièreConnexion")
        .send()
        .await?;
    r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

    let mut json_response: Value = r.json().await?;

    if json_response.get("redirect").is_some()
    {
        get_cookies(client).await.context("Erreur lors de la récupération des cookies")?;

        let r: Response = client.get("https://notes.iut-larochelle.fr/services/data.php?q=dataPremièreConnexion")
            .send()
            .await?;
        r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

        json_response = r.json().await?;
    }

    for semestre in json_response["semestres"].as_array().context("Erreur lors de la récupération des semestres")?
    {
        // Si le semestre n'est pas le semestre de dataPremièreConnexion, on l'ajoute à la liste des semestres à récupérer
        if (semestre["semestre_id"].as_u64().context("Erreur lors de la récupération des ids des semestres")?) > 2 && 
		semestre["formsemestre_id"] != json_response["relevé"]["formsemestre_id"]
        {
            semestres_to_fetch.push(semestre["formsemestre_id"].as_u64().context("Erreur lors de la récupération des ids des semestres")?);
        }

    }
    let releve: &Value = &json_response["relevé"];




// On récupère les données du semestre de dataPremièreConnexion
    for ue in releve["ues"].as_object().context("Erreur lors de la récupération des ues")?
    {
        let grade: &str = ue.1["moyenne"]["value"].as_str().context("Erreur lors de la récupération des moyennes")?;
        if grade.parse::<f32>().is_ok()
        {
            // Ces trous de balle ont mis la moyenne en string, donc on récupère le str puis on parse en float
            ues_grades.push(grade.parse::<f32>().context("Erreur lors de la récupération des moyennes")?);
        } else
        {
            ues_grades.push(0.0);
        }

    }

    for sae in releve["saes"].as_object().context("Erreur lors de la récupération des saes")?
    {
        let mut sae_note: f32 = 0.0;
        for evaluation in sae.1["evaluations"].as_array().context("Erreur lors de la récupération des notes de saes")?
        {
            let note: &str = evaluation["note"]["value"].as_str().context("Erreur lors de la récupération de la note de sae")?;
            if note.parse::<f32>().is_ok()
            {
                sae_note += note.parse::<f32>().context("Erreur lors de la récupération de la moyenne de la sae")?;
            }
        }
        map_saes.insert(sae.0.to_string(), sae_note);
    }

    for ressource in releve["ressources"].as_object().context("Erreur lors de la récupération des ressources")?
    {
        let mut ressource_note: f32 = 0.0;
        for evaluation in ressource.1["evaluations"].as_array().context("Erreur lors de la récupération des notes de ressources")?
        {
            let note: &str = evaluation["note"]["value"].as_str().context("Erreur lors de la récupération de la note de ressource")?;
            if note.parse::<f32>().is_ok()
            {
                ressource_note += note.parse::<f32>().context("Erreur lors de la récupération de la moyenne de la ressource")?;
            }
        }
        map_ressources.insert(ressource.0.to_string(), ressource_note);
    }




// On récupère les données des autres semestres
    for id in &semestres_to_fetch
    {

        let r: Response = client.get("https://notes.iut-larochelle.fr/services/data.php?q=relevéEtudiant&semestre=".to_owned()+id.to_string().as_str())
            .send()
            .await?;
        r.error_for_status_ref().context("Erreur lors de la récupération des données")?;

        let json_response: Value = r.json().await?;
        let releve: &Value = &json_response["relevé"];




        for ue in releve["ues"].as_object().context("Erreur lors de la récupération des ues")?
        {
            let grade_index: usize =  usize::try_from(ue.0.chars().last().context("Erreur lors de la récupération de l'ue")?.to_digit(10).context("Erreur lors de la récupération de l'ue")?).context("Erreur lors de la récupération de l'ue")?;
            let grade: &str = ue.1["moyenne"]["value"].as_str().context("Erreur lors de la récupération des moyennes")?;
            let value_to_add: f32;
            if grade.parse::<f32>().is_ok()
            {
                value_to_add = grade.parse::<f32>().context("Erreur lors de la récupération des moyennes")?.clone();
            } else {
                value_to_add = ues_grades[grade_index - 1];
            }
            ues_grades[grade_index - 1] += value_to_add;
        }

        for sae in releve["saes"].as_object().context("Erreur lors de la récupération des saes")?
        {
            let mut sae_note: f32 = 0.0;
            for evaluation in sae.1["evaluations"].as_array().context("Erreur lors de la récupération des notes de saes")?
            {
                let note: &str = evaluation["note"]["value"].as_str().context("Erreur lors de la récupération de la note de sae")?;
                if note.parse::<f32>().is_ok()
                {
                    sae_note += note.parse::<f32>().context("Erreur lors de la récupération de la moyenne de la sae")?;
                }
            }
            map_saes.insert(sae.0.to_string(), sae_note);
        }

        for ressource in releve["ressources"].as_object().context("Erreur lors de la récupération des ressources")?
        {
            let mut ressource_note: f32 = 0.0;
            for evaluation in ressource.1["evaluations"].as_array().context("Erreur lors de la récupération des notes des ressources")?
            {
                let note: &str = evaluation["note"]["value"].as_str().context("Erreur lors de la récupération de la note de ressource")?;
                if note.parse::<f32>().is_ok()
                {
                    ressource_note += note.parse::<f32>().context("Erreur lors de la récupération de la moyenne de la ressource")?;
                }
            }
            map_ressources.insert(ressource.0.to_string(), ressource_note);
        }
    }


    ues_grades = ues_grades.iter().map(|x| x/f32::from(i16::try_from(semestres_to_fetch.len()+1).unwrap())).collect();

    println!("{:?}", ues_grades);

    return Ok(ues_grades);

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

    let json_payload = json!({
        "content": null,
        "embeds": [
            {
                "title": "Changement détecté",
                "description": format!("Une modification a été détectée sur **{}**{} ", payload_str, if year_valid { "\n 🟢 Tu **as** ton année" } else { "\n 🔴 Tu **n'as pas** ton année" }),
                "color": 5814783
            }
        ],
        "attachments": []
    });

    client
        .post(&webhook_url)
        .json(&json_payload)
        .send()
        .await?;

    println!("Webhook envoyé");

    Ok(())
}

async fn get_is_year(ues: &Vec<f32>) -> Result<bool> {
    let mut get_is_year : bool = true;
    let mut count_below_10: i8 = 0;

    for ue_nb in ues {
        if *ue_nb < 8.0 {
            get_is_year = false;
        }
        if *ue_nb < 10.0 {
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

    let client: Client = Client::builder().danger_accept_invalid_certs(true).cookie_store(true).build()?;


    get_cookies(&client).await?;
    // Effectuer une requête GET pour récupérer la page de connexion



    let mut old_vec_saes: HashMap<String, f32> = HashMap::new();
    let mut old_vec_ressources: HashMap<String, f32> = HashMap::new();
    let mut a_vec: Vec<String>;
    let mut new_vec_ressources: HashMap<String, f32> = HashMap::new();
    let mut new_vec_saes: HashMap<String, f32> = HashMap::new();

    get_ues_and_data(&client, &mut old_vec_saes, &mut old_vec_ressources).await?;

    loop
    {
        println!("{:?}", chrono::offset::Local::now());
        println!("refresh...");
        let ues_grades: &Vec<f32> = &get_ues_and_data(&client, &mut new_vec_saes, &mut new_vec_ressources).await?;

        a_vec = compare_dictionnaries(&old_vec_saes, &new_vec_saes);
        if !a_vec.is_empty()
        {
            println!("{:?}", a_vec);

            //on envoie le webhook
            send_webhook(a_vec, get_is_year(ues_grades).await?).await?;

            //on change l'ancien dictionnaire par le nouveau
            old_vec_saes = new_vec_saes.clone();
        }
        a_vec = compare_dictionnaries(&mut old_vec_ressources, &mut new_vec_ressources);
        if !a_vec.is_empty()
        {
            println!("{:?}", a_vec);

            //on envoie le webhook
            send_webhook(a_vec, get_is_year(ues_grades).await?).await?;

            //on change l'ancien dictionnaire par le nouveau
            old_vec_ressources = new_vec_ressources.clone();
        }
        thread::sleep(Duration::from_secs(300));
    }


}
