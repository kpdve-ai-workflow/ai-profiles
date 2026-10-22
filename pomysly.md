- obecnie nie ma mechanizmu pozwalajacego wrocic do konkretnej wersji profilu
- brak hooka git, ktory zablokuje wypchniecie zmian gdy nie zrobimy bump wersji ( lub zastanowic sie nad automatycznym wersojonowaniem gdy wdrozymy jakies zmiany w profilach )
- brak hooka git conventional commits ( nie przepuszczamy commitow gd nie spelniaja tego wymogu )
- zastanowic sie nad systemem zarzadzania agentami z wykorzystaniem herdr ( dodatkowo w tym uzyc agents ) ale nie tylko, w skrocie jakis ala systemik na poziomie systemu operacyjnego ( W RUST ) ktory pozwoli w dobry sposob zarzadzac procesami herdr ( o ile sam herdr nie jest wstanie tego robic ) po prostu na poziomie systemu operacyjnego tak aby jak najmniej umieszczac w skillach bo wiadomo ze AI halucynuje ;]
- dodac blokade na bump wersji lub jakos bardziej pokierowac agenta ( obecny problem, trakuje mala zmianke jako major wersje )
- agent przy blednym pluginie sie zacinal w momencie aktualizacji pliku .md ( nie widac bylo w TUI po prostu ciagle pracowal jakis podlgad by sie przydal )
- config dostarczany juz do repo jako profil jest za duzy, zbyt wiele plikow laduje po instalacji pomyslec jak mozna to uproscic
- zapomnialem o docker build po wdrozeniu mcp playwright, przydaloby sie moze stworzyc jakies lepsze instrukcje + automatyczne tworzenie folderu reports/wcag ( wywalic wcag i dodac bardziej generyczny folder ) w trakcie buildu docker
- rozdzielic instalacje na 3 ( osobno opencode codex claude code + opcja wszystko )

# Bugi
To pozostałości po testach instalatora, nie błąd ścieżki WFOSiGW. Potwierdziłem je w ~/.codex/config.toml.
Co się stało:
1. Test mcp/images/playwright/tests/run.sh:69 uruchamia prawdziwe make install w tymczasowym projekcie, bez izolacji konfiguracji Codexa.
2. Zostawił globalnie trzy marketplace’y wskazujące na /tmp/opencode/playwright-smoke....
3. Codex próbuje je teraz wczytać, ale nie znajduje poprawnych manifestów. Instalator zatrzymuje aktywację już przy codex plugin marketplace list --json, zanim zarejestruje WFOSiGW.
Pliki profilu w WFOSiGW zostały przygotowane, ale aktywacja w Codexie nie została ukończona. Komunikat o Claude jest niezależny — nie masz zainstalowanego claude CLI.
Jak odblokować instalację
W ~/.codex/config.toml usuń wyłącznie sekcje [marketplaces.…] oraz odpowiadające im [plugins."ai-profile@…"] dla:
- ai-profiles-6b64be70771c
- ai-profiles-4150094c9470
- ai-profiles-dfcca2d5b049
Zachowaj wpis ai-profiles-b84ecae0ddbe — wskazuje na rzeczywisty projekt react.
Następnie uruchom z repozytorium ai-profiles:
make update PROJECT=/var/home/kpdve/it/personal/wfosigw-ssh/WFOSiGW
Docelowo trzeba też odizolować konfigurację Codexa w teście Playwright, żeby sytuacja nie wracała. Nic jeszcze nie zmieniałem.
