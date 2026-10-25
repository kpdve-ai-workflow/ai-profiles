# Zarządzanie profilami i instalacjami

## Tworzenie i struktura profilu

Profile są wykrywane automatycznie z `profiles/<nazwa>/`. Profil zawiera
`profile.json`, `mcp.json` oraz `skills.json`. `name` w metadanych musi
odpowiadać nazwie katalogu. Najprościej utworzyć profil poleceniem:

```bash
make create PROFILE=backend-python DESCRIPTION="Backend Python development profile"
```

Nazwa jest wymagana; opis jest opcjonalny. Komenda tworzy metadane w wersji
`1.0.0`, pusty wybór MCP i pusty wybór skilli. Nie instaluje
profilu ani nie nadpisuje istniejącego. Szczegóły formatu wyboru MCP znajdują się
w [dokumentacji MCP](../mcp/README.md).

Skille dystrybuowane przechowuj raz w `skills/<nazwa>/SKILL.md`, z frontmatter
`name` i `description`; nazwa musi odpowiadać katalogowi. Pliki pomocnicze
pozostają w katalogu skilla. Profile wybierają je jawnie w `skills.json`.
Przykładowy wybór WordPress:

```json
{
  "skills": ["acf", "make-plan", "sage-block-scaffold", "styling"]
}
```

Nazwy są bezpośrednimi nazwami katalogów pod `skills/`; wpisy muszą być
unikalne. Dwa profile mogą wskazać ten sam katalog źródłowy, a instalator
kopiuje każdy skill do własnego projektu. Na przykład
`skills/make-plan/SKILL.md` trafia do
`<projekt>/.ai-profiles/<klient>/plugin/skills/make-plan/SKILL.md`, a
`skills/make-plan/references/plan-layout.md` do tego samego katalogu
`references/`. Każdy klient otrzymuje oddzielną kopię dystrybucyjną.
Nowy profil zaczyna z `{"skills":[]}`. Zmiany manifestu docierają do projektu
przez `make update CLIENT=<klient> PROJECT=<projekt>`; usunięte pliki są zdejmowane zgodnie z
kontrolą własności i hashy. Nie używaj symlinków ani kolidujących nazw.

Instalacja przygotowuje osobny payload wybranego klienta. OpenCode otrzymuje jawny
source skills w `opencode.json`, a wybrane Claude Code lub Codex dostają natywne katalogi
projektowe oraz włączenie pluginu. Katalog Claude znajduje się w
`.claude-plugin/marketplace.json`, katalog Codex w
`.agents/plugins/marketplace.json`. Identyfikator marketplace zawiera skrót
kanonicznej ścieżki projektu, dzięki czemu projekty nie dzielą tożsamości.

Przed użyciem pluginu Claude Code może wymagać zaufania repozytorium; Codex
ładuje `.codex/config.toml` tylko w zaufanym projekcie. Instalator nie zmienia
zaufania ani uprawnień. Gdy CLI hosta nie ma, pliki projektu są przygotowane,
a aktywacja pozostaje oczekująca. Przy błędzie aktywacji `status` pokazuje
oddzielnie pliki projektu i ostatni zapisany wynik instalacji natywnej; ponowne
`make update CLIENT=codex PROJECT=<projekt>` ponawia aktywację Codex po błędzie lub stanie oczekującym.
Claude Code instaluje plugin przez `claude plugin install ai-profile@<marketplace>
--scope project`; przy kolejnej aktualizacji menedżer używa `claude plugin
update`.

Codex rejestruje lokalny marketplace i instaluje plugin osobnymi poleceniami,
uruchamianymi z katalogu projektu:

```bash
codex plugin marketplace add <bezwzględna-ścieżka-projektu> --json
codex plugin add ai-profile@<marketplace-z-.agents/plugins/marketplace.json> --json
```

Instalator bierze nazwy z katalogu Codex i weryfikuje rejestrację oraz wynik
przez `codex plugin list --marketplace <marketplace> --json`. Istniejąca
rejestracja jest używana tylko wtedy, gdy jej nazwa i lokalne źródło pasują do
projektu. Rejestracja i instalacja Codex zapisują ustawienia oraz cache w
konfiguracji użytkownika Codex; CLI nie udostępnia tu opcji zakresu projektu.
Stan śledzi, które zasoby natywne dodał instalator, a `uninstall` usuwa tylko te
zasoby. Istniejące pluginy, rejestracje i jawne wyłączenie pozostają zachowane.

Powtórne `make update CLIENT=codex PROJECT=<projekt>` sprawdza inwentarz i nie powtarza `plugin add`, jeśli
plugin jest już zainstalowany. Zmiana plików projektu bez zmiany wersji nie
dowodzi odświeżenia zawartości cache Codex; nie potwierdzono obsługiwanego
polecenia odświeżającego lokalny plugin, więc aktualność cache pozostaje
niezweryfikowana. Wynik `installed` oznacza wyłącznie potwierdzenie przez
natywny inwentarz. Nie dowodzi, że uruchomiona sesja Codex załadowała skille.
Uruchom ponownie Codex, aby ocenić je w nowej sesji. Wpisy `Dir` lub `File` w
selektorze wzmianek nie potwierdzają wykrycia skilla.

Skill może się załadować, gdy jego wymagany serwer MCP nie jest dostępny w tym
hoście. Instalator przygotowuje MCP tylko w konfiguracji wybranego klienta:
`opencode.json`, `.codex/config.toml` albo `.mcp.json`, na podstawie wspólnego rejestru i lokalnych
nadpisań. Status opisuje konfigurację; nie sprawdza połączenia MCP. Po instalacji
lub aktualizacji uruchom ponownie klienta, zatwierdź projekt i MCP zgodnie z
jego natywnymi ustawieniami, a zdalne OAuth zaloguj osobno w każdym kliencie.
Skille mogą też zachowywać metadane
`opencode/slash`, ale składnia wywołania i automatyczna aktywacja zależą od hosta.

Starszy współdzielony payload `.opencode/` pozostaje zamrożony, również gdy
wybrany klient przechodzi do izolowanego układu. Nie jest automatycznie usuwany.
Narzędzia użytkownika profilu umieszczaj w `profiles/<profil>/scripts/`, nie
w skillach. Są instalowane bez automatycznego wykonywania.

Skille przeznaczone wyłącznie dla tego repozytorium umieszczaj w
`.opencode/skills/`; OpenCode wykrywa je natywnie. Każdy katalog skilla jest
udostępniany Codexowi przez odpowiadający mu symlink w `.agents/skills/`.
Codex wykrywa repozytoryjne skille z `.agents/skills/` i obsługuje symlinkowane
katalogi skilli. Dzięki temu treść ma jedno źródło i zachowuje pliki pomocnicze.
Przy dodawaniu lub usuwaniu skilla utrzymuj symlink w zgodzie ze źródłem;
sprawdza to `bash tests/check-codex-skills.sh`. Te skille nie są częścią
`skills/` ani żadnego profilu i nie są dostarczane do projektów. Workflow tego
checkoutu jest oddzielony od zawartości profili.

Źródło: [OpenAI Codex — Build skills](https://developers.openai.com/codex/build-skills/).
Codex wymaga w `SKILL.md` pól `name` i `description`; obecne skille OpenCode
spełniają ten format.

## Wersje i publikowanie

Nazwę istniejącego profilu zmieniaj poleceniem:

```bash
make rename PROFILE=stara-nazwa NEW_NAME=nowa-nazwa
```

Komenda zmienia katalog `profiles/stara-nazwa` na `profiles/nowa-nazwa`
oraz pole `name` w jego `profile.json`. Zachowuje wersję, opis, dodatkowe
metadane, skille, skrypty i wybór MCP. Nie zastępuje nazw w treści dokumentacji.
Nowa nazwa musi być wolna i składać się z małych liter/cyfr rozdzielonych
pojedynczymi myślnikami. Nie używa `PROJECT` i odrzuca opcję `--project`.
Odpowiednik CLI: `ai-profile rename stara-nazwa --new-name nowa-nazwa`.

Istniejące instalacje pozostają bez zmian i nadal wskazują starą nazwę.
Komenda nie tworzy aliasu; ich `status` i `update` mogą odrzucić profil,
którego stara nazwa nie istnieje już w źródłach.

Każdy profil ma niezależną stabilną wersję `MAJOR.MINOR.PATCH`. Aktualizuj ją
poleceniem `make bump PROFILE=<nazwa> PART=patch|minor|major` (domyślnie `patch`):

- **MAJOR** — niekompatybilna zmiana;
- **MINOR** — kompatybilne rozszerzenie;
- **PATCH** — poprawka istniejących instrukcji lub konfiguracji.

Wersje prerelease i build metadata nie są obsługiwane. Zmiana plików nie zwiększa
wersji automatycznie. Zmiany wspólne wymagają świadomego bumpu wszystkich
dotkniętych profili; przy zmianie definicji MCP bumpuj tylko profile, które ją
wybierają. Bump zmienia wyłącznie metadane: nie aktualizuje projektów, nie
wykonuje commitów ani nie tworzy tagów.

## Instalacja i aktualizacja

Instaluj przez `make install CLIENT=opencode|claude|codex|all PROFILE=<profil> PROJECT=<projekt>`.
Aliasy: `install-opencode`, `install-claude`, `install-codex` z tymi samymi PROFILE
i PROJECT; sprzeczny CLIENT jest błędem. Make buduje
manager i runtime Rust w `core/rust/target/release/` przed operacją. Adapter MCP,
launcher Playwright i restore WordPress trafiają do projektu jako samodzielne
pliki wykonywalne. Python pozostaje wyłącznie w repozytorium jako kod
referencyjny. Współdzielone zasoby legacy (również dawne `.py`) pozostają zamrożone;
obce i lokalnie zmienione pliki zachowują dotychczasową ochronę.

Make wymaga `PROFILE` przy `create`, `rename`, `install`, `switch` i `bump`, oraz `PROJECT`
przy komendach projektowych. CLI domyślnie działa na bieżącym katalogu.
`status` i `update` korzystają z lokalnego checkoutu i nie pobierają zmian;
`pull-update` wykonuje `git pull --ff-only`, a następnie `update`. Przed oceną
najnowszej wersji upstream pobierz zmiany jawnie.

`update` dodaje i synchronizuje zarządzane pliki oraz usuwa wycofane pliki,
zachowując pliki obce i lokalną konfigurację. Powtórzenie bez zmian daje tę samą
zawartość. Downgrade tego samego profilu jest blokowany; `switch` wybiera inny
profil. `uninstall` usuwa wyłącznie zarządzaną zawartość.

Stan `.ai-profiles/state.json` formatu 2 śledzi osobno dla każdego klienta wersję, źródło, SHA-256 plików,
zarządzane wpisy MCP i dane potrzebne do bezpiecznej synchronizacji. Status
rozróżnia aktualność wersji od zmian treści oraz pokazuje lokalne modyfikacje.
SHA commit nie opisuje niezacommitowanych zmian; do publikowania używaj czystego
checkoutu.

Stary stan `.opencode/ai-profile-state.json` oraz współdzielone zasoby legacy
pozostają zachowane, ponieważ inni klienci lub cache pluginów mogą ich używać.
Wpis `workflow-rules.md` w `instructions` jest usuwany przy update/switch
wyłącznie, gdy stan potwierdza, że dodał go instalator (`instructions_added`).
Własny wpis użytkownika pozostaje; po usunięciu wpisu instalatora flaga
ownership jest zerowana.

Przed zapisem menedżer sprawdza hashe plików, wpisy MCP i kolizje ścieżek.
Konflikt przerywa operację bez częściowego zastosowania; nie ma opcji wymuszającej
nadpisanie lokalnych zmian. Zrób kopię lub commit, przenieś zmiany wspólne do
repozytorium, a projektowe ustawienia MCP do `.ai-profiles/<klient>/local.json`.
Przywróć zarządzany plik/wpis do zainstalowanej wartości i ponów aktualizację.
Nie edytuj hashy stanu ani zarządzanych wpisów w `opencode.json`. Menedżer odmawia
pracy z `opencode.jsonc` tylko dla operacji OpenCode, by nie utracić komentarzy. Zapis nie jest transakcją
odporną na awarię dysku lub przerwanie procesu.

## Migracja starszej instalacji

Format 1 jest importowany leniwie; status niczego nie zapisuje. Pierwsza udana
operacja zmieniająca stan zapisuje format 2. Dodanie klienta nie przenosi innych.
Tylko update/switch wybranego klienta zmienia jego żywe odwołania na
`.ai-profiles/<klient>/`. Odpowiednie stare nadpisania są kopiowane do local.json
bez zmiany oryginału; konflikt z docelowym plikiem blokuje operację.
Stary stan i współdzielone pliki nie są automatycznie sprzątane. Pusty manifest
formatu 2 blokuje ponowny import starego stanu.

Stan formatu 1 bez wersji/SHA commita jest obsługiwany: status pokaże
`VERSION UNKNOWN`, a udany update uzupełni metadane po sprawdzeniu hashy.
Starsze stany bez `mcp_base` są również aktualizowane po sprawdzeniu starej
konfiguracji. Instalacje MVP bez hashy wymagają przeglądu i backupu przed
świadomym przejęciem:

```bash
make migrate-legacy PROJECT=~/projects/my-api
make update CLIENT=opencode PROJECT=~/projects/my-api
```

Migracja uznaje istniejące pliki za punkt odniesienia. Walidację repozytorium
uruchom przez `make validate`; szczegóły walidacji MCP są w dokumentacji MCP.

Starsze instalacje aktualizuj osobno przez `make update CLIENT=<klient> PROJECT=<projekt>`.
Brakującą integrację dodaj przez `make install CLIENT=<klient> PROFILE=<profil> PROJECT=<projekt>`.
Przygotowanie konfiguracji nie wymaga obecności
CLI tych klientów. Dla Playwright instalator dostarcza launcher, Dockerfile i Compose
do `.ai-profiles/<klient>/mcp/playwright/`, bez uruchamiania Dockera lub npm. Przygotowanie
obrazu wykonuje użytkownik w katalogu głównym docelowego repozytorium:

```sh
docker build --tag ai-profiles-playwright:0.2.0 .ai-profiles/opencode/mcp/playwright
```

Dla Claude Code/Codex użyj w ścieżce odpowiednio `claude`/`codex`.
Do korzystania potrzebny jest lokalny Linux Docker Engine i plugin Compose.
Adapter uruchamia dostarczony `playwright-launcher`, który automatycznie tworzy
`.ai-profiles/reports/` i przygotowuje kontener, bez konfiguracji VPN lub hosta.
Nie ma automatycznego buildu/pull ani zależności od publikacji GHCR.
Zależności Playwright są w obrazie, bez hostowego Node. Opcjonalny Basic Auth
używa `<PROJECT>/base-auth.json` z `username`, `password`, `origin` i prawami
`0600`; bez pliku działa bez uwierzytelniania. Opcjonalne cookies/storage state
także wskazuj jako plik w katalogu projektu, z prawami `0600`. Compose nie wymaga edycji.
Po przygotowaniu zrestartuj klienta i zatwierdź wymagane natywne zgody projektu/MCP.

React 4.0.0 i WordPress 3.1.0 używają `.ai-profiles/reports/`; poświadczenia
użytkownik tworzy w katalogu głównym projektu, bez migracji starych lokalizacji.
Sieć pozostaje `host`.
WordPress dodatkowo przygotowuje `.ai-profiles/<klient>/mcp/novamira/` z oficjalnym ZIP-em,
skryptem instalacji pluginu i mostem w Dockerze. Ścieżki Novamiry są stałe;
wszystkie `env_file` MCP są ścieżkami względnymi od katalogu głównego projektu.
`<PROJECT>/.env.novamira` jest wymagany wyłącznie przy uruchamianiu Novamiry;
użytkownik tworzy go samodzielnie, instalator go nie dostarcza. `env_file` MCP
może wskazywać ścieżkę względną od katalogu projektu.
Uninstall nie usuwa credentials ani reguły `.gitignore`, którą instalator dodaje
dla domyślnego pliku Basic Auth. Instrukcja i wyjątki (CA, uprawnienia, HTTP 401)
są w `.ai-profiles/<klient>/mcp/playwright/README.md`;
[szczegóły wspólne](../mcp/README.md#playwright-i-audyty-wcag).
