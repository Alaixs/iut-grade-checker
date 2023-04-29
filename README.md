
# IUT Grade checker

Un simple script en python qui check toute les 5 minutes si il ya une note et si vous avez votre année par webhook discord

## Contenu

Il y a deux versions :
```
- Global : main.py 
Prévu pour toute les plateformes
```

```
Raspberry pi OS lite : main-rpiOSlite.py
Prévu pour rasberry pi tournant sous RPI OS Lite, elle permet le fonctionnement du script tout en ayant une interface "headless"
```

## Installation 

Vous avez besoin du fichier python (main.py ou main-rpiOSlite.py au choix)

Vous devez récupérer le fichier **.env** et **changer les variables avec vos valeurs**.

Executer le script avec 
```
python main.py
ou
python main-rpiOSlite.py
```
Ou en background avec 
```
nohup python main.py &
ou 
nohup python main-rpiOSlite.py &
```
