# Testy

Testy menedżera są grupowane według odpowiedzialności: profile i payload,
system plików, operacje, status, walidacja, migracja, CLI oraz konfiguracja i
runtime MCP. Integracje WordPress i launcher MariaDB mają osobne zestawy.

Testy używają tymczasowych kopii repozytorium i katalogów projektów; nie należy
uruchamiać ich na prawdziwych projektach. Kopie powinny zawierać źródła potrzebne
CLI i Make, bez `.git`, cache i samych testów. Git, procesy zewnętrzne i granice
runtime należy mockować tam, gdzie wynik nie powinien zależeć od środowiska.

Pełny zestaw: `make test` lub `python3 -m unittest discover -s tests -v`.
Pojedynczy moduł: `python3 -m unittest discover -s tests -p 'test_operations.py' -v`.
