Zrobiłem przegląd statyczny ścieżek instalacji, konfiguracji MCP, uruchamiania i usuwania profilu. Załącznik obejmuje 38 plików Rusta, około 11 tys. wierszy, ale brakuje diffu, manifestu Cargo, treści testów oraz kluczowych plików Docker i skryptów. Nie zmieniałem kodu i nie uruchamiałem testów. Odwołania do modułów poniżej dotyczą sekcji w załączonym zrzucie źródeł.    Pasted text
1. Najważniejsze konkretne ustalenia
A. Aktualizacja dostarczonych plików Novamiry zostanie potraktowana jak konflikt
To najważniejsze ograniczenie utrzymaniowe, które widzę.

W profile_management::credentials masz:
- pliki dostarczane przez program, osadzone przez include_bytes!;
- stały adres ZIP-a oraz jego SHA-256;
- funkcję check(), która porównuje istniejące pliki z aktualną wersją osadzoną w binarce;
- funkcję seed(), która tworzy tylko brakujące pliki.
Jeżeli istniejący launcher, Dockerfile czy compose.yaml różni się od aktualnie dostarczanego, check() odmawia działania. Analogicznie istniejący ZIP musi odpowiadać aktualnej stałej SHA.

Z tego wynika następujący scenariusz:
1. Instalujesz profil z wersją A launchera.
2. W nowej wersji ai-profiles poprawiasz launcher.
3. Użytkownik uruchamia aktualizację profilu.
4. Poprawnie zainstalowana wersja A różni się od nowej wersji B.
5. Instalator zgłasza konflikt, tak jak przy ręcznej modyfikacji użytkownika.
Kod nie ma informacji pozwalającej odróżnić „poprzednią zarządzaną wersję” od „pliku zmodyfikowanego przez użytkownika”. Wspólny zestaw plików Novamiry jest poza zwykłym wykazem record.files, z którego korzysta mechanizm własności i hashy.    Pasted text

To nie oznacza, że pierwsza instalacja jest błędna. Twój prompt nie definiuje szczegółowo aktualizacji tego zestawu. Jest to jednak bardzo konkretna przyczyna przyszłych problemów: nawet drobna poprawka skryptu wymaga rozstrzygnięcia, jak wymienić wcześniej dostarczony plik.

Co bym zmienił w modelu: rozdzieliłbym zasoby dostarczane przez ai-profiles od danych użytkownika.
Kategoria	Przykłady	Potrzebna polityka
Zasoby zarządzane	Dockerfile, Compose, launcher, install-plugin, przypięty ZIP	Znana zainstalowana wersja; kontrolowana aktualizacja; wykrywanie lokalnych modyfikacji
Dane użytkownika	.env.novamira, prywatne materiały	Utworzenie, jeśli brakuje; zachowanie zawartości; brak hashów sekretów w stanie
Zewnętrzna instalacja	Plugin w WordPressie	Osobna, jawna operacja instalacji; brak usuwania przez uninstall profilu


Zakaz nadpisywania poświadczeń nie wymaga, żeby również kod dostarczany przez ai-profiles był na zawsze niezmienny. Te zasady powinny mieć osobnych właścicieli.
B. Wspólny ZIP jest przygotowywany osobno dla każdego klienta
Tutaj jest bardzo czytelny przykład źle umieszczonej odpowiedzialności.

profile_management::scoped::Prepared reprezentuje operację dla jednego klienta. Ma jednak pole:
novamira_archive: Option<Vec<u8>>

Przepływ wygląda następująco:
1. run_with_runner() przygotowuje operacje wszystkich wybranych klientów.
2. Każde prepare() dla profilu z Novamirą wywołuje credentials::archive_for().
3. Jeśli ZIP-a jeszcze nie ma, archive_for() pobiera go do pamięci.
4. Dopiero po przygotowaniu wszystkich klientów rozpoczyna się commit().
5. Zapis ZIP-a na dysk następuje w commit().
Przy świeżej instalacji dla wszystkich trzech klientów, bez istniejącego ZIP-a i przy pomyślnych pobraniach, ten sam ZIP zostanie pobrany trzy razy. Wynika to z kolejności wywołań; nie jest to wynik przeprowadzonego pomiaru. Dla Twojego przykładu z samym CLIENT=opencode będzie jedno pobranie.    Pasted text

To dobrze pokazuje problem architektury:
Zasób należący do projektu został umieszczony w planie pojedynczego klienta.

Najmniejsza sensowna poprawa to nadrzędny plan instalacji zawierający wspólne przygotowanie Novamiry oraz osobne plany klientów.

Ważne: zachowałbym istniejącą zasadę, że sprawdzenia poprzedzają zapisy. Przeniesienie zapisu ZIP-a do pierwszego prepare() tylko po to, żeby kolejne wywołania znalazły plik, osłabiłoby tę zasadę. Lepiej przygotować wspólny zasób raz i przekazać go do planu całej operacji.
C. Błąd w trakcie zapisu może pozostawić instalację, której nie da się zwyczajnie ponowić
W Prepared::commit() kolejność jest następująca:
1. przygotowanie wspólnych plików Novamiry;
2. usuwanie starych plików;
3. zapis nowego payloadu;
4. zapis konfiguracji klienta;
5. aktualizacja i zapis manifestu.
Jeśli wystąpi błąd I/O po części zapisów, ale przed utrwaleniem manifestu, część dysku będzie odpowiadała nowej instalacji, a stan nadal starej — albo nie będzie go wcale.

Przy ponowieniu filesystem::check_files() może wtedy zgłosić:
- UnmanagedFileExists, bo plik istnieje, ale nie ma go w zapisanym stanie;
- ManagedFileChanged, bo zawartość różni się od starego hasha.
W tej ścieżce nie widzę rollbacku ani mechanizmu wznowienia operacji. Atomowy zapis samego manifestu nie obejmuje wcześniejszych zapisów innych plików.    Pasted text

To jest luka w odzyskiwaniu po awarii wynikająca z przepływu kodu, bez wykonanej reprodukcji. Dotyczy operacji jednego klienta. Jest czymś innym niż świadoma rezygnacja z jednej transakcji obejmującej wszystkich klientów.

Nie wymagałbym od razu rozbudowanego systemu transakcyjnego. Wymagałbym natomiast określenia i przetestowania jednego zachowania: po przerwaniu zapisu instalator albo wraca do poprzedniego stanu, albo potrafi rozpoznać częściową operację i ją dokończyć.
2. Co w tej architekturze powoduje rozchodzenie się zmian
A. Novamira jest jednocześnie modułem i zestawem wyjątków w innych modułach
Część jej zachowania jest dobrze wydzielona w mcp::novamira. Ten moduł obsługuje szczególny sposób inicjalizacji mostu, rozpoznawanie błędu połączenia i komunikację stdio.

Jednak wiedza o tej samej integracji występuje również tutaj:
Miejsce	Co wie o Novamirze?
mcp::config::load_local_at()	Których pól nie wolno nadpisywać i gdzie mają leżeć zasoby
mcp::runtime::launch()	Jakie zmienne WordPressa są wymagane i kiedy uruchomić specjalną obsługę inicjalizacji
profile_management::scoped::prepare()	Kiedy sprawdzić zestaw plików i pobrać ZIP
profile_management::scoped::Prepared::commit()	Kiedy zapisać ten zestaw
profile_management::credentials	URL, SHA, pliki Docker, launcher, instalator pluginu, katalogi i szablon poświadczeń


Wspólny kod wielokrotnie sprawdza nazwę "novamira" i na tej podstawie wykonuje szczególne działania. Podobny wzorzec widać przy PostgreSQL i Playwright.    Pasted text

To tłumaczy kaskadę zmian: żeby zmienić jedną integrację, trzeba pamiętać o kilku fazach jej życia, rozłożonych między ogólnymi modułami.

Sama zmiana nazwy credentials.rs niczego nie naprawi, ale jego obecna zawartość jest dobrym sygnałem diagnostycznym. Moduł nazwany „poświadczenia” odpowiada też za dystrybucję pluginu i środowiska uruchomieniowego.

Docelowo chciałbym jednego miejsca odpowiedzialnego za Novamirę, które zna:
- przypięty artefakt;
- zestaw dostarczanych plików;
- wymagania środowiska;
- stałe ścieżki;
- szczególne zachowanie inicjalizacji;
- zasady aktualizacji i zachowania danych.
Ogólny instalator powinien otrzymywać od niego opis potrzebnych działań. Na początek wystarczą zwykłe funkcje, struktury i jedno miejsce wyboru integracji. Nie widzę potrzeby budowy frameworka pluginów ani dużej hierarchii traitów.
B. Typowany model MCP istnieje, ale dalszy przepływ wraca do surowego JSON-a
To moim zdaniem najważniejsza uwaga specyficzna dla wykorzystania Rusta w tym projekcie.

Masz McpServerDefinition z typowanymi polami, m.in.:
- McpTransport;
- McpTimeout;
- McpOAuth;
- McpProtocol;
- komendą, środowiskiem i ścieżką pliku env.
parse_server() buduje ten model. Następnie jednak:
- definitions_to_map() bierze przede wszystkim definition.raw.clone();
- resolve_overrides() wywołuje validate_servers(), ale odrzuca zwrócony typowany wynik;
- runtime ponownie odczytuje "type", "command", "cwd", "enabled" i "env_file" z Value;
- tłumaczenie konfiguracji do klientów ponownie interpretuje te pola.
Masz koszt utrzymywania typowanego modelu oraz koszt wielokrotnego ręcznego interpretowania tego samego schematu.    Pasted text

Dodanie nowej właściwości MCP może przez to wymagać pamiętania o:
1. parserze;
2. dozwolonych override’ach;
3. scalaniu;
4. normalizacji;
5. renderowaniu konfiguracji klientów;
6. uruchomieniu procesu.
Kompilator nie wskaże wszystkich pominiętych miejsc, bo wiele zależności jest zapisanych jako klucze tekstowe.

Tutaj zastosowałbym prostą granicę:
Odczyt i scalanie JSON-a → walidacja i normalizacja → typowana konfiguracja wewnętrzna → wykonanie lub zapis konfiguracji klienta.

Po etapie normalizacji runtime powinien już dostawać informację, że to lokalny serwer z konkretną komendą, zamiast kolejny raz sprawdzać, czy pole "command" jest tablicą stringów.

Nie oznacza to konieczności usunięcia serde_json::Value z całego projektu. Przy zachowywaniu nieznanych pól cudzej konfiguracji jest ono użyteczne. Ważne, żeby znane pola nie miały dwóch konkurencyjnych reprezentacji wewnątrz silnika.

To zmienia akcent mojej wcześniejszej porady: w tym fragmencie projektu bardziej wykorzystałbym typy jako wewnętrzny kontrakt, zamiast skupiać się przede wszystkim na builderach czy redukowaniu generyków.
C. Własny stan instalacji też jest rozproszonym schematem tekstowym
Podobny problem występuje w manifeście instalacji.

project::manifest_for() czyta i interpretuje stan jako Value. scoped::Manifest przechowuje mapę wartości JSON. scoped::prepare() składa rekord przez json!, a commit() zna szczegółowe klucze stanu aktywacji poszczególnych klientów.

Przykładowo orkiestrator wie o nazwach takich jak:
- settings_had_file;
- config_had_file;
- installed_before;
- activation_reason;
- last_successful_sync.
Zmiana znaczenia takiego pola wymaga znalezienia jego producentów i konsumentów w kilku modułach.    Pasted text

Dla własnego formatu stanu wprowadziłbym spójne typy, np. odpowiadające stanowi projektu, klienta i wspólnych zasobów. Odczyt oraz zapis JSON-a byłyby granicą tego modelu.

Metadane konkretnego klienta powinny pozostać pod kontrolą jego adaptera. Orkiestrator może wiedzieć, że aktywacja zakończyła się powodzeniem, oczekuje na działanie użytkownika albo się nie udała. Nie powinien potrzebować znajomości każdej flagi specyficznej dla Claude’a czy Codexa.

Przy takim porządkowaniu trzeba zachować obecny dobry podział dotyczący sekretów: chwilowe środowisko uruchamianego procesu nie powinno stać się serializowalnym stanem instalacji.
D. Nowe ścieżki powstają przez przepisywanie starych ścieżek
W payload::build_files() wiele ścieżek jest najpierw budowanych pod .opencode/....

Później:
- scoped::files_for() przepisuje ich lokalizację;
- layout::managed_path() zna stare prefiksy;
- layout::render() wykonuje zamiany w treści;
- scoped::render_mcp() osobno zna ścieżki MCP i nazwy plików Compose Playwrighta.
Novamira otrzymała jeszcze inną drogę: bezpośrednie stałe .ai-profiles/mcp/novamira/... oraz pliki osadzone w binarce.    Pasted text

W efekcie odpowiedź na pytanie „gdzie powinien trafić ten zasób?” zależy od kilku modułów i od etapu przetwarzania.

Stopniowo doprowadziłbym do tego, żeby nowy plan od razu otrzymywał właściwe lokalizacje:
- zasobów wspólnych projektu;
- zasobów wybranego klienta;
- natywnej konfiguracji klienta.
Obsługa historycznych ścieżek może pozostać na granicy odczytu starego formatu, jeśli jest potrzebna. Używanie ich jako pośredniego języka nowych instalacji zwiększa liczbę reguł, o których trzeba pamiętać.
3. Co już jest dobrze zaprojektowane
Nie widzę powodu, żeby przepisywać ten projekt od zera. Są tu elementy, na których można oprzeć poprawę:

Podział prepare / commit. To dobry kierunek: wykrywasz konflikty przed rozpoczęciem zapisu. Warto rozszerzyć go o wspólne zasoby i odzyskiwanie po błędzie.

Konkretny PayloadFile. To prosty model opisujący plik do dostarczenia. Podobny poziom prostoty zachowałbym w dalszym planowaniu operacji.

ProcessRunner jako punkt podmiany wykonywania procesów. To uzasadniona abstrakcja związana z efektami ubocznymi i testowaniem. Nie wygląda na niepotrzebne uogólnienie całej aplikacji.

Mały RepositoryContext. Przenosi konkretną informację o źródłach. Nie stał się obiektem zawierającym wszystkie usługi i konfiguracje programu.

Osobna obsługa specyfiki inicjalizacji Novamiry. mcp::novamira ma spójną odpowiedzialność. Przeniósłbym ją pod wspólną granicę integracji, zachowując sam pomysł wydzielenia.

Ostrożne tworzenie nowego pliku poświadczeń. W bootstrapie używasz create_new(true) i trybu 0600; istniejąca zawartość nie jest odczytywana ani nadpisywana przez ten etap.    Pasted text

Ważne jest też uniknięcie błędnej diagnozy: historyczny installation::apply_profile() ma #[cfg(test)] i komentarz wskazujący użycie do starych fixtures. Zwykłe operacje CLI trafiają do scoped::run(). Nie są to dwa równoległe produkcyjne instalatory. Historyczne fixtures mogą zwiększać koszt utrzymania, ale nie przypisywałbym im większej roli bez treści testów.
4. Jak kod wypada względem Twojego promptu
Poniżej rozdzielam mechanizmy widoczne w Rust od elementów, których załącznik nie pozwala sprawdzić.
Wymaganie	Ocena na podstawie materiału
Pobranie przypiętej paczki	Mechanizm obecny. Stały URL wydania v1.12.12, stały SHA-256, limit wielkości i timeout
Sprawdzenie integralności	Mechanizm obecny. Pobrane bajty są porównywane z oczekiwanym SHA; istniejący ZIP również jest sprawdzany
Zasoby pod .ai-profiles/mcp/novamira	Widoczne w bootstrapie. Są stałe ścieżki i lista dostarczanych plików
Nowy .env.novamira z 0600, bez nadpisywania	Potwierdzone w kodzie. Powstaje szablon z pustymi wartościami
Identyfikacja WordPressa i faktycznego katalogu pluginów	Niezweryfikowane. Brakuje treści install-plugin
Instalacja pluginu bez niejawnego nadpisywania	Niezweryfikowane. Również zależy od brakującego skryptu
Node i przypięty most wyłącznie w Dockerze	Niezweryfikowane. Brakuje Dockerfile’a, Compose, launchera i definicji serwera
Brak automatycznej konfiguracji AI Abilities i tworzenia poświadczeń	W widocznym przepływie instalacji Rust takich operacji nie ma. Skrypt pluginu wymaga osobnego przeglądu
Brak sekretów w stanie instalacji	Zawartość tego .env nie jest zapisywana przez widoczny bootstrap do stanu. Pozostałe warstwy wymagają pełnego przeglądu
Uninstall zachowuje poświadczenia i plugin WordPressa	Widoczna ścieżka Rust ich nie usuwa. Pozostawia również wspólny zestaw plików Novamiry
Izolacja klientów	Mechanizm per klient jest obecny. Są osobne rekordy i konfiguracje; pełne zachowanie wymaga testów
Rzeczywiste testy Docker i MCP	Nie mogę potwierdzić. Widać odwołania do modułów testowych i nazwy plików na screenach, ale nie ich treść ani wyniki


Ustalenia o implementacji powyżej wynikają z załączonego kodu.    Pasted text

Sprawdziłem też oficjalne źródło: strona pobierania Novamiry odsyła do repozytorium GitHub i wymienia wersję 1.12.12; istnieje odpowiadające jej wydanie. Nie pobierałem jednak ZIP-a, więc nie potwierdzam zgodności wpisanej stałej SHA z rzeczywistym archiwum ani jego kompletności. novamira.ai

Są jeszcze trzy niuanse:

„Pusty .env” oznacza w implementacji szablon bez poświadczeń. Plik zawiera komentarze i puste wartości zmiennych. Uważam to za rozsądną interpretację promptu, o ile nie wymagałeś literalnie pliku o długości zero.

Prywatne uprawnienia są ustawiane przy tworzeniu. Istniejący .env jest sprawdzany pod względem typu pliku, ale bootstrap nie weryfikuje, czy nadal ma 0600. Jeśli wymaganie ma obowiązywać również dla istniejących plików, potrzebna jest walidacja i czytelny komunikat. Nie wymaga to odczytywania zawartości sekretów.

Ogólna obsługa legacy nadal istnieje. Widać ją w Layout, odczycie starego stanu i wybranych operacjach scoped. Nie znalazłem podstaw, żeby uznać samo jej istnienie za migrację starej lokalizacji Novamiry. Bez diffu nie wiadomo też, czy była częścią tego zadania. Jeśli zakaz migracji miał obejmować cały instalator, zakres ten nie jest obecnie zrealizowany; jeśli dotyczył nowej integracji, należy oceniać go węziej.

Podobnie komentarz o npx w kodzie sprzątania procesów nie dowodzi uruchamiania hostowego npx, a obecność wordpress::ddev nie dowodzi, że instalacja Novamiry startuje DDEV. Te wnioski wymagałyby rzeczywistej ścieżki wywołań.
5. Czy plików na screenach jest za dużo?
Znaczna część tej listy jest naturalną konsekwencją zakresu.

Dla takiej funkcji spodziewałbym się zmian w:
- definicji serwera MCP;
- module przygotowania zasobów;
- konfiguracji uruchamiania;
- Dockerfile’u i Compose;
- launcherze;
- instalatorze pluginu;
- instrukcjach profilu i skilla;
- testach pobierania, instalacji, inicjalizacji i zachowania sekretów.
Dopisanie modułu testowego w lib.rs, aktualizacja README czy dodanie nowych plików obrazu nie wskazuje samo w sobie na sprzężenie.

Natomiast na screenach są też zmiany dotyczące Playwrighta, profilu React i skilla wcag-audit. Dla nich poprosiłbym w review o związek z wymaganiem. Mogą wynikać ze wspólnej polityki ścieżek, aktualizacji zależnego profilu albo poprawki regresji, ale sama lista nazw tego nie pokazuje. Nie ma podstaw, żeby uznać je automatycznie za zbędne.  image  image(1)

Lepsze kryterium niż liczba plików brzmi:
Czy zmiana konkretnego szczegółu Novamiry wymaga ponownego przemyślenia niezależnych integracji i konfiguracji klientów?

Jeżeli tak, sprawdzałbym granicę odpowiedzialności. Jeżeli diff obejmuje kod nowej funkcji, jej zasoby, testy i dokumentację, szerokość zmiany może być całkowicie uzasadniona.

Warto też rozdzielać w zadaniach trzy rzeczy: dostarczenie funkcji, refaktor wspólnego mechanizmu oraz poprawki poboczne. Każda może być potrzebna, ale ich połączenie utrudnia ocenę, które pliki były wymagane przez pierwotny prompt.
6. Jaką architekturę proponowałbym dla tego projektu
Zachowałbym obecny crate. Najpierw uporządkowałbym odpowiedzialności w jego wnętrzu.
Obszar	Odpowiedzialność
Integracja Novamira	Artefakt ZIP, pliki środowiska, wymagane zmienne, szczególna inicjalizacja, polityka aktualizacji
Adapter konkretnego klienta	Przygotowanie jego konfiguracji, własne metadane, aktywacja i sprzątanie
Plan instalacji projektu	Wspólne zasoby przygotowane raz oraz lista planów klientów
Wykonanie planu	Sprawdzenie konfliktów, zapisy, aktualizacja stanu, odzyskiwanie po błędzie
Model stanu	Własny, typowany format zapisanej instalacji
Konfiguracja MCP	Odczyt, scalanie i normalizacja do jednego modelu używanego dalej
Lokalizacje projektu	Jasne rozróżnienie zasobów projektu, klienta i natywnej konfiguracji


Najważniejsze są tu dwie granice.
Granica pierwsza: wspólne zasoby i konfiguracje klientów
Plan całej operacji powinien wiedzieć, że Novamira jest potrzebna, i przygotować jej wspólne zasoby raz.

Następnie plan OpenCode, Claude’a czy Codexa powinien dostać informacje potrzebne do swojej konfiguracji. Nie powinien ponownie pobierać wspólnego ZIP-a ani samodzielnie rozstrzygać zasad jego aktualizacji.

To dokładnie odpowiada problemowi, który już dziś widać przy CLIENT=all.
Granica druga: kontrakt integracji i kontrakt klienta
Novamira odpowiada na pytanie:
„Co trzeba przygotować i jak uruchomić tę integrację?”

Adapter klienta odpowiada na pytanie:
„Jak zapisać to w konfiguracji tego klienta i zachować jego istniejące ustawienia?”

Instalator koordynuje te odpowiedzi.

W obecnym kodzie scoped::prepare() i commit() wiedzą zbyt wiele o obu stronach. Samo rozcięcie dużego native.rs na trzy mniejsze pliki da ograniczony efekt, jeśli orkiestrator nadal będzie składał metadane i szczegóły konfiguracji każdego klienta.

Przy trzech znanych klientach mały, centralny match Client jest zupełnie rozsądny. Ważne, żeby wybierał kompletną operację adaptera, zamiast zawierać coraz więcej jego szczegółów.
7. Kolejność zmian, którą wybrałbym praktycznie
Etap 1: uporządkować wspólny zestaw Novamiry
Najpierw wydzieliłbym przygotowanie wspólnych zasobów z planu pojedynczego klienta.

Kryterium zakończenia: świeża instalacja dla wszystkich klientów pobiera ZIP raz, a konflikt wykryty podczas przygotowania klienta nie powoduje przedwczesnego zapisu pozostałych zasobów.

To jest mały, konkretny refaktor z obserwowalnym skutkiem.
Etap 2: określić aktualizację zasobów zarządzanych
Dodałbym rozróżnienie między:
- poprzednią wersją dostarczoną przez ai-profiles;
- lokalną modyfikacją;
- danymi użytkownika.
Kryterium zakończenia: poprawka launchera może zostać dostarczona istniejącej instalacji zgodnie z ustaloną polityką, przy zachowaniu .env i prywatnych danych. Ręczna modyfikacja zarządzanego pliku nadal powoduje czytelny konflikt.

To nie wymaga migracji do innej lokalizacji. Chodzi o cykl życia plików w obecnym katalogu.
Etap 3: wykorzystać typowany wynik normalizacji MCP
Przestałbym odrzucać wynik validate_servers() i ograniczył ponowne interpretowanie znanych pól JSON-a.

Kryterium zakończenia: runtime oraz renderery klientów korzystają ze wspólnego modelu rozstrzygniętej konfiguracji. Nie muszą osobno sprawdzać podstawowego kształtu tych samych pól.
Etap 4: zawęzić rolę orkiestratora
Przeniósłbym składanie specyficznych konfiguracji i metadanych za interfejsy adapterów klientów. Równolegle uporządkowałbym typy własnego stanu.

Kryterium zakończenia: zmiana szczegółu konfiguracji jednego klienta nie wymaga modyfikowania kilku miejsc w scoped, które ręcznie znają jego klucze.
Osobno: zapewnić odzyskiwanie po częściowym zapisie
Ten temat potraktowałbym jako zmianę zachowania instalatora, z testem wymuszonego błędu I/O. Nie chowałbym go w kosmetycznym refaktorze.

Najbardziej wartościowe testy dla tej serii zmian to:
- wspólne pobranie dokładnie raz;
- aktualizacja poprzedniego zarządzanego zestawu;
- zachowanie lokalnych modyfikacji;
- zachowanie zawartości sekretów przy ponownej instalacji i uninstall;
- ponowienie operacji po kontrolowanym przerwaniu zapisu;
- pełne uruchomienie mostu z rzeczywistym WordPressem, gdy dostępne będą brakujące skrypty i środowisko.
Mój pierwszy ruch w Twoim repo byłby więc bardzo konkretny: oddzielić wspólny cykl życia Novamiry od instalacji pojedynczego klienta. Następnie uporządkowałbym przepływ typowanej konfiguracji MCP. Te dwie zmiany mają bezpośrednie uzasadnienie w pokazanym kodzie i powinny ograniczyć liczbę miejsc, o których trzeba pamiętać przy następnej integracji.

Do rozstrzygnięcia, czy wszystkie zmiany ze screenów były potrzebne, potrzebny będzie diff tego konkretnego zadania. Do wskazania pierwszych granic wymagających poprawy obecny materiał już wystarcza.