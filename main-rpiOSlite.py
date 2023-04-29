import time
import os
import json
import requests
from dotenv import load_dotenv
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from pyvirtualdisplay import Display

##################
# INITIALISATION #
##################

#Chemin de ChromeDriver
chrome_path = "/usr/lib/chromium-browser/chromedriver"

#Initialisation de Selenium WebDriver headless
service = Service(chrome_path)
display = Display(visible=0, size=(800, 600))
display.start()
driver = webdriver.Chrome(service=service)

load_dotenv()
username_ent = os.getenv('USERNAME_ENT')
password_ent = os.getenv('PASSWORD_ENT')
wehbhook_url = os.getenv('WEBHOOK_URL')

payload_discord = {
  "embeds": [
    {
      "title": "Nouvelle note disponible !",
      "description": "",
      "color": 43287
    }
  ],
  "attachments": []
}


    ###########################
	# 	  Payload Discord 	  #
	###########################

def send_discord_message(get_is_year):
    if get_is_year:
        payload_discord["embeds"][0]["description"] = "👨‍🎓 🟢 Actuellement tu as ton année !"
    else:
        payload_discord["embeds"][0]["description"] = "👨‍🎓 🔴 Actuellement tu n'as pas ton année !"
    requests.post(wehbhook_url, json=payload_discord)

def get_all_cookies(driver):
    ###########################
	# 	Cookie de l'ENT lr	  #
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
	# 	Cookie de notes IUT   #
	###########################
    url_notes_id = "https://notes.iut-larochelle.fr/"
    driver.get(url_notes_id)

    wait = WebDriverWait(driver, 10)
    wait.until(EC.invisibility_of_element_located(
        (By.XPATH, '//body[@class="hideAbsences etudiant"]/div[@class="auth" and @style="opacity: 0; pointer-events: none;"]')))


    ###########################
	# 	Récupérer les notes   #
	###########################
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
    return ue_averages


url_semestre2 = "https://notes.iut-larochelle.fr/services/data.php?q=relev%C3%A9Etudiant&semestre=401"
url_semestre1 = "https://notes.iut-larochelle.fr/services/data.php?q=relev%C3%A9Etudiant&semestre=357"

ue_list_semestre2 = ["UE 2.1", "UE 2.2", "UE 2.3", "UE 2.4", "UE 2.5", "UE 2.6"]
ue_list_semestre1 = ["UE 1.1", "UE 1.2", "UE 1.3", "UE 1.4", "UE 1.5", "UE 1.6"]


    ###########################
	# Check si année validée  #
	###########################
old_total = 0
def check_note():
    count_below_10 = 0
    for ue_nb in range(0, 6):
        res = float(ue_averages_semestre1[ue_nb]) + float(ue_averages_semestre2[ue_nb])
        res /= 2
        if res < 8:
            send_discord_message(False)
            break
        else:
            if ue_nb == 5:
                send_discord_message(True)
        if res < 10:
            count_below_10 += 1
    if count_below_10 > 2:
        send_discord_message(False)

  ###########################
	#    Boucle principale    #
	###########################

while True:
    ue_averages_semestre2 = get_ue_averages(driver, url_semestre2, ue_list_semestre2)
    ue_averages_semestre1 = get_ue_averages(driver, url_semestre1, ue_list_semestre1)
    pre = WebDriverWait(driver, 10).until(
    EC.presence_of_element_located((By.TAG_NAME, "pre")))
    for i in range(0, 6):
        actual_total = float(ue_averages_semestre2[i]) + float(ue_averages_semestre1[i])
    if actual_total != old_total:
        check_note()
        old_total = actual_total
    time.sleep(300)
