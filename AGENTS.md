Instalacja odbywa się poprzez `make install CLIENT=opencode|claude|codex|all PROFILE=<profil> PROJECT=<projekt>`.

## Obowiązkowa polityka testowania

- Przed projektowaniem, dodawaniem, modyfikowaniem lub oceną testów przeczytaj [docs/testing.md](docs/testing.md) i stosuj metodykę `test-gap-discovery`; wczytaj skill, jeśli jest dostępny. Gdy jest niedostępny, zgłoś to i nadal stosuj metodykę opisaną w tym dokumencie, zamiast zastępować ją dowolnym podejściem.
- Zmiana zachowania aplikacji wymaga adekwatnej ochrony testowej i wykonania wymaganej weryfikacji zgodnie z `docs/testing.md`. Obowiązuje to również bez Brain i handoffu.
- Brak środowiska, uprawnień lub możliwości wykonania wymaganych testów blokuje weryfikację; nie oznacza sukcesu. Zatrzymaj pracę i zgłoś blokadę, zachowując informację o wykonanych zmianach.
- Nie osłabiaj testów, nie pomijaj wymaganych kontroli ani nie obchodź izolacji, aby zakończyć zadanie. Wymagana VM dotyczy testów Rust i integracji Playwright, nie lokalnych operacji produkcyjnych, takich jak instalacja profilu.
