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
`<projekt>/.opencode/ai-profile/plugin/skills/make-plan/SKILL.md`, a
`skills/make-plan/references/plan-layout.md` do tego samego katalogu
`references/`. To jedyna kopia dystrybucyjna skilla w projekcie.
Nowy profil zaczyna z `{"skills":[]}`. Zmiany manifestu docierają do projektu
przez `make update PROJECT=<projekt>`; usunięte pliki są zdejmowane zgodnie z
kontrolą własności i hashy. Nie używaj symlinków ani kolidujących nazw.

Instalacja przygotowuje wspólny plugin `ai-profile`. OpenCode otrzymuje jawny
source skills w `opencode.json`, a Claude Code i Codex dostają natywne katalogi
projektowe oraz włączenie pluginu. Katalog Claude znajduje się w
`.claude-plugin/marketplace.json`, katalog Codex w
`.agents/plugins/marketplace.json`. Identyfikator marketplace zawiera skrót
kanonicznej ścieżki projektu, dzięki czemu projekty nie dzielą tożsamości.

Przed użyciem pluginu Claude Code może wymagać zaufania repozytorium; Codex
ładuje `.codex/config.toml` tylko w zaufanym projekcie. Instalator nie zmienia
zaufania ani uprawnień. Gdy CLI hosta nie ma, pliki projektu są przygotowane,
a aktywacja pozostaje oczekująca. Przy błędzie aktywacji `status` pokazuje
oddzielnie pliki projektu i ostatni zapisany wynik instalacji natywnej; ponowne
`make update` ponawia aktywację Codex po błędzie lub stanie oczekującym.
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

Powtórne `make update` sprawdza inwentarz i nie powtarza `plugin add`, jeśli
plugin jest już zainstalowany. Zmiana plików projektu bez zmiany wersji nie
dowodzi odświeżenia zawartości cache Codex; nie potwierdzono obsługiwanego
polecenia odświeżającego lokalny plugin, więc aktualność cache pozostaje
niezweryfikowana. Wynik `installed` oznacza wyłącznie potwierdzenie przez
natywny inwentarz. Nie dowodzi, że uruchomiona sesja Codex załadowała skille.
Uruchom ponownie Codex, aby ocenić je w nowej sesji. Wpisy `Dir` lub `File` w
selektorze wzmianek nie potwierdzają wykrycia skilla.

Skill może się załadować, gdy jego wymagany serwer MCP nie jest dostępny w tym
hoście. Instalator przygotowuje wybrane MCP w `opencode.json`,
`.codex/config.toml` oraz `.mcp.json` na podstawie wspólnego rejestru i lokalnych
nadpisań. Status opisuje konfigurację; nie sprawdza połączenia MCP. Po instalacji
lub aktualizacji uruchom ponownie klienta, zatwierdź projekt i MCP zgodnie z
jego natywnymi ustawieniami, a zdalne OAuth zaloguj osobno w każdym kliencie.
Skille mogą też zachowywać metadane
`opencode/slash`, ale składnia wywołania i automatyczna aktywacja zależą od hosta.

Przy aktualizacji instalacji ze starszego układu pliki z
`.opencode/skills/<nazwa>/` są usuwane dopiero po zgodności z zapisanymi hashami.
Lokalnie zmieniony zarządzany plik blokuje migrację; niezależne skille projektu
pozostają nietknięte.
Narzędzia użytkownika profilu umieszczaj w `profiles/<profil>/scripts/`, nie
w skillach. Są instalowane bez automatycznego wykonywania.

Skille przeznaczone wyłącznie dla tego repozytorium umieszczaj w
`.opencode/skills/`; OpenCode wykrywa je natywnie. Nie są częścią `skills/`
ani żadnego profilu i nie są dostarczane do projektów. Workflow tego checkoutu
jest oddzielony od zawartości profili.

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

Instaluj przez `make install PROFILE=<profil> PROJECT=<projekt>`. Make buduje
manager i runtime Rust w `core/rust/target/release/` przed operacją. Adapter MCP,
launcher MariaDB i restore WordPress trafiają do projektu jako samodzielne
pliki wykonywalne. Python pozostaje wyłącznie w repozytorium jako kod
referencyjny. Aktualizacja usuwa dawne zarządzane `.py` po sprawdzeniu hashy;
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

Stan `.opencode/ai-profile-state.json` śledzi wersję, źródło, SHA-256 plików,
zarządzane wpisy MCP i dane potrzebne do bezpiecznej synchronizacji. Status
rozróżnia aktualność wersji od zmian treści oraz pokazuje lokalne modyfikacje.
SHA commit nie opisuje niezacommitowanych zmian; do publikowania używaj czystego
checkoutu.

Wycofany `.opencode/workflow-rules.md` oraz dawniej wspólnie dostarczany
`brain-to-implementer` są usuwane z istniejącej instalacji podczas update/switch
lub uninstall tylko wtedy, gdy ich hashe zgadzają się ze stanem instalatora.
Wpis `workflow-rules.md` w `instructions` jest usuwany przy update/switch
wyłącznie, gdy stan potwierdza, że dodał go instalator (`instructions_added`).
Własny wpis użytkownika pozostaje; po usunięciu wpisu instalatora flaga
ownership jest zerowana.

Przed zapisem menedżer sprawdza hashe plików, wpisy MCP i kolizje ścieżek.
Konflikt przerywa operację bez częściowego zastosowania; nie ma opcji wymuszającej
nadpisanie lokalnych zmian. Zrób kopię lub commit, przenieś zmiany wspólne do
repozytorium, a projektowe ustawienia MCP do `.opencode/ai-profile.local.json`.
Przywróć zarządzany plik/wpis do zainstalowanej wartości i ponów aktualizację.
Nie edytuj hashy stanu ani zarządzanych wpisów w `opencode.json`. Menedżer odmawia
pracy z `opencode.jsonc`, by nie utracić komentarzy. Zapis nie jest transakcją
odporną na awarię dysku lub przerwanie procesu.

## Migracja starszej instalacji

Stan formatu 1 bez wersji/SHA commita jest obsługiwany: status pokaże
`VERSION UNKNOWN`, a udany update uzupełni metadane po sprawdzeniu hashy.
Starsze stany bez `mcp_base` są również aktualizowane po sprawdzeniu starej
konfiguracji. Instalacje MVP bez hashy wymagają przeglądu i backupu przed
świadomym przejęciem:

```bash
make migrate-legacy PROJECT=~/projects/my-api
make update PROJECT=~/projects/my-api
```

Migracja uznaje istniejące pliki za punkt odniesienia. Walidację repozytorium
uruchom przez `make validate`; szczegóły walidacji MCP są w dokumentacji MCP.

Starsze instalacje zyskają konfigurację Codex i Claude Code przez
`make update PROJECT=<projekt>`. Przygotowanie konfiguracji nie wymaga obecności
CLI tych klientów. Playwright WCAG wymaga działającego Dockera; przy starcie MCP
launcher automatycznie pobiera brakujący obraz
`ghcr.io/kpdve-ai-workflow/playwright:0.1.0`. Instalator nie uruchamia npm,
nie instaluje hostowych przeglądarek i nigdy nie buduje obrazu. Pierwsze wydanie
obrazu pozostaje nieopublikowane, więc do czasu publikacji pull zakończy się
czytelnym błędem. Procedura maintainerów znajduje się w `mcp/README.md`.
