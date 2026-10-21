# Umiejętności dystrybuowane z profilami

Ten katalog jest centralnym źródłem umiejętności dostarczanych do projektów
przez menedżer profili. Każdy podkatalog, na przykład
`skills/make-plan/`, zawiera jedną umiejętność: jej instrukcję `SKILL.md` oraz
opcjonalne materiały pomocnicze, takie jak szablony, skrypty i pliki w
`references/`.

Umiejętność jest instalowana tylko wtedy, gdy jej nazwa znajduje się w tablicy
`skills` pliku `profiles/<profil>/skills.json`. Wpis wskazuje bezpośrednio
podkatalog w tym katalogu. Dzięki wspólnemu źródłu wiele profili może wybierać
tę samą umiejętność, a każdy projekt otrzymuje własną zarządzaną kopię w
lokalizacji umiejętności obsługiwanej przez używane narzędzie.

Zmiany w instrukcji lub materiałach pomocniczych oraz zmiany wyboru profilu
docierają do zainstalowanego projektu po uruchomieniu
`make update PROJECT=<projekt>`. Ten katalog nie obejmuje umiejętności
lokalnych dla tego repozytorium, które nie są dystrybuowane z profilami.
