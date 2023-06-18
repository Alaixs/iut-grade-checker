
# IUT Grade Checker

Un simple script python qui vérifie toutes les 5 minutes s'il y a une note et si vous avez votre année en envoyant un webhook discord.

## Contenu

Il n'y a qu'une seule version :
```
- main.py
- .env
- requirements.txt
- readme.md
```

## Installation 

Commencez par cloner le repo

```
git clone https://github.com/Alaixs/iut-grade-checker.git
cd iut-grade-checker
```

Dans le fichier **.env** vous devez **changer les variables avec vos valeurs**.

Installez le fichier requirement.txt afin d'avoir toutes les dépendances.
```
pip install -r requirements.txt
```

## Exécuter le script

Exécuter le script avec 
```
python main.py
```
Ou en arrière-plan avec 

```
nohup python main.py &
```
