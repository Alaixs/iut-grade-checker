import os
import requests
import pytz
import time
import datetime
import warnings
from dotenv import load_dotenv
from bs4 import BeautifulSoup
from urllib3.exceptions import InsecureRequestWarning



####################
#  REQUESTS SETUP  #
####################

warnings.filterwarnings("ignore", category=InsecureRequestWarning)
LOGIN_URL = \
    'https://authentification.univ-lr.fr/cas/login?service=' \
    'https://notes.iut-larochelle.fr/services/doAuth.php?href=' \
    'https://notes.iut-larochelle.fr/'


load_dotenv()
username_ent = os.getenv("USERNAME_ENT")
password_ent = os.getenv("PASSWORD_ENT")
wehbhook_url = os.getenv("WEBHOOK_URL")
nb_request = 0
tz = pytz.timezone('Europe/Paris')

####################
#  DISCORD MESSAGE #
####################

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



####################
#  SAVE FUNCTIONS  #
####################

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

####################
# CHECK NOTE PART  #
####################

def get_ue_averages(url, ue_list):

    s = requests.Session()

    pre_auth = s.get(LOGIN_URL, verify=False)
    soup = BeautifulSoup(pre_auth.text, 'lxml')
    exec_value = soup.select_one('section.cas-field:not(.my-3)>input[name="execution"]').get('value')

    form_data = {'username': username_ent, 'password': password_ent, 'execution': exec_value,
                 '_eventId': 'submit', 'geolocation': ''}
    s.post(LOGIN_URL, data=form_data, verify=False)

    r = s.get(url, verify=False)
    parsed_json = r.json()

    ue_averages = [parsed_json["relevé"]["ues"][ue]["moyenne"]["value"] for ue in ue_list]
    save_request_checked()
    return ue_averages


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

####################
#  MAIN FUNCTIONS  #
####################

def main():
    url_semestre2 = "https://notes.iut-larochelle.fr/services/data.php?q=relev%C3%A9Etudiant&semestre=401"
    url_semestre1 = "https://notes.iut-larochelle.fr/services/data.php?q=relev%C3%A9Etudiant&semestre=357"

    ue_list_semestre2 = ["UE 2.1", "UE 2.2", "UE 2.3", "UE 2.4", "UE 2.5", "UE 2.6"]
    ue_list_semestre1 = ["UE 1.1", "UE 1.2", "UE 1.3", "UE 1.4", "UE 1.5", "UE 1.6"]

    old_total = 0

    while True:
        # register the last try to do a request
        save_request_try()
        # get the averages of the 2 semesters
        ue_averages_semestre2 = get_ue_averages(url_semestre2, ue_list_semestre2)
        ue_averages_semestre1 = get_ue_averages(url_semestre1, ue_list_semestre1)
        # calculate the total average
        actual_total = sum(float(avg) for avg in ue_averages_semestre2 + ue_averages_semestre1)

        with open('last_note.txt', 'r', encoding='utf-8') as f:
            # register the last note in case of crash
            last_note = f.read()
        # if the total average is different from the last one AND the last note is not the same as the actual one
        if actual_total != old_total and last_note != str(actual_total):
            # check if the student has his year
            check_note(ue_averages_semestre1, ue_averages_semestre2)
            # register the new note
            old_total = actual_total
            with open('last_note.txt', 'w', encoding='utf-8') as f:
                f.write(str(actual_total))

        # increment the number of request (data)
        increment_request()
        print("refresh")
        print(old_total, actual_total)

        # wait 5 minutes
        time.sleep(300)


if __name__ == "__main__":
    main()