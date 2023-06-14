import os
import json
import requests
import pytz
import time
import datetime
from dotenv import load_dotenv
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import NoSuchWindowException
from selenium.common.exceptions import NoSuchElementException


load_dotenv()
username_ent = os.getenv("USERNAME_ENT")
password_ent = os.getenv("PASSWORD_ENT")
wehbhook_url = os.getenv("WEBHOOK_URL")
nb_request = 0
tz = pytz.timezone('Europe/Paris')

payload_discord = {
    "embeds": [
        {"title": "Nouvelle note disponible !", "description": "", "color": 43287}
    ],
    "attachments": [],
}

def send_discord_message(get_is_year):
    print("send discord message")
    if get_is_year:
        payload_discord["embeds"][0][
            "description"
        ] = "👨‍🎓 🟢 Actuellement tu as ton année !"
    else:
        payload_discord["embeds"][0][
            "description"
        ] = "👨‍🎓 🔴 Actuellement tu n'as pas ton année !"
    requests.post(wehbhook_url, json=payload_discord)

def get_all_cookies(driver):
    ###########################
    #   Cookie de l'ENT lr    #
    ###########################
    url_general_id = "https://authentification.univ-lr.fr/cas/login"
    driver.get(url_general_id)

    driver.implicitly_wait(10)

    # Récupérer les éléments HTML correspondant aux champs d'identification
    username_input = driver.find_element(By.NAME, "username")
    password_input = driver.find_element(By.NAME, "password")

    # Entrer les informations d'identification dans les champs appropriés
    username_input.send_keys(username_ent)
    password_input.send_keys(password_ent)
    # Soumettre le formulaire de connexion
    password_input.submit()

    ###########################
    #   Cookie de notes IUT   #
    ###########################
    url_notes_id = "https://notes.iut-larochelle.fr/"
    driver.get(url_notes_id)

    wait = WebDriverWait(driver, 10)
    wait.until(
        EC.invisibility_of_element_located(
            (
                By.XPATH,
                '//body[@class="hideAbsences etudiant"]/div[@class="auth" and @style="opacity: 0; pointer-events: none;"]',
            )
        )
    )

def get_ue_averages(driver, url, ue_list):
    driver.get(url)
    texte = driver.find_element(By.TAG_NAME, "pre").text
    # Si besoin d'auth, on récupère les cookies
    if '{"redirect":"\/services\/doAuth.php"}' in texte:
        get_all_cookies(driver)
        driver.get(url)
    content = driver.find_element(By.TAG_NAME, "pre").text
    parsed_json = json.loads(content)
    ue_averages = [parsed_json["relevé"]["ues"][ue]["moyenne"]["value"] for ue in ue_list]
    save_request_checked()
    return ue_averages

def save_request_try():
    with open('last_request_try.txt', 'w', encoding='utf-8') as f:
        f.write(str(datetime.datetime.now(tz)))

def save_request_checked():
    with open('last_request_checked.txt', 'w', encoding='utf-8') as f:
        f.write(str(datetime.datetime.now(tz)))

def increment_request():
    with open('nb_request.txt', 'r', encoding='utf-8') as f:
        nb_request = int(f.read())
    nb_request += 1
    with open('nb_request.txt', 'w', encoding='utf-8') as f:
        f.write(str(nb_request))

def check_note(ue_averages_semestre1, ue_averages_semestre2):
    count_below_10 = 0
    for ue_nb in range(0, 6):
        res = float(ue_averages_semestre1[ue_nb]) + float(ue_averages_semestre2[ue_nb])
        res /= 2
        print(count_below_10)
        if res < 10:
            count_below_10 += 1
        if res < 8 or count_below_10 > 2:
            print("UE", ue_nb + 1, "average:", res, "pas validé")
            send_discord_message(False)
            return
    send_discord_message(True)
    print("Semestre 1 et 2 validés")

def main():
    chrome_path = "/usr/lib/chromium-browser/chromedriver"
    options = webdriver.ChromeOptions()
    options.add_argument("--headless")
    options.add_argument("disable-infobars")
    options.add_argument("--disable-extensions")
    options.add_argument("--disable-gpu")
    options.add_argument("--disable-dev-shm-usage")
    options.add_argument("--no-sandbox")
    service = Service(chrome_path)
    driver = webdriver.Chrome(service=service, options=options)

    url_semestre2 = "https://notes.iut-larochelle.fr/services/data.php?q=relev%C3%A9Etudiant&semestre=401"
    url_semestre1 = "https://notes.iut-larochelle.fr/services/data.php?q=relev%C3%A9Etudiant&semestre=357"

    ue_list_semestre2 = ["UE 2.1", "UE 2.2", "UE 2.3", "UE 2.4", "UE 2.5", "UE 2.6"]
    ue_list_semestre1 = ["UE 1.1", "UE 1.2", "UE 1.3", "UE 1.4", "UE 1.5", "UE 1.6"]

    old_total = 0

    while True:
        save_request_try()
        try:
            ue_averages_semestre2 = get_ue_averages(driver, url_semestre2, ue_list_semestre2)
            ue_averages_semestre1 = get_ue_averages(driver, url_semestre1, ue_list_semestre1)
            actual_total = sum(float(avg) for avg in ue_averages_semestre2 + ue_averages_semestre1)

            with open('last_note.txt', 'r', encoding='utf-8') as f:
                last_note = f.read()
                save_request_try()
            if actual_total != old_total and last_note != str(actual_total):
                check_note(ue_averages_semestre1, ue_averages_semestre2)
                old_total = actual_total
                with open('last_note.txt', 'w', encoding='utf-8') as f:
                    f.write(str(actual_total))

            print("refresh")
            print(old_total, actual_total)

            time.sleep(300)

        except NoSuchWindowException:
            print("Window closed. Reopening...")
            driver.quit()
            driver = webdriver.Chrome(service=service, options=options)

if __name__ == "__main__":
    main()