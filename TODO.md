# TODO — nedokončená/rozpracovaná místa (stav `feat/config`, 2026-10-14)

Sepsáno po průchodu celým `src/` (1015 řádků) před PR do `master`. Řazeno podle závažnosti.

## 🔴 Bugy / bezpečnost — řešit před mergem

- [ ] **Auth je úplně vyřazená.** `Token::from_request` ([src/auth.rs:11](src/auth.rs#L11)) naparsuje Bearer token do `_token`, ale vrací vždy `Token("".to_string())` — hodnota se nikam nepropaguje. `post_article` ([src/articles.rs:177](src/articles.rs#L177)) token přijme jako `_token: Token` a dál ho ignoruje, `validate_token()` se nikde nevolá (celé je zakomentované, řádky 179–186 a 183–186). Efekt: `POST /api/v1/article` je dnes bez ochrany.
  - Rozhodnout: buď dotáhnout OIDC validaci (odkomentovat a zapojit `validate_token()`), nebo auth vědomě vypnout a smazat mrtvý OIDC kód (`oidc_jwt_validator` dependency, `OIDC_URL`, `validate_token`), aby nepůsobil dojem funkční featury.
  - Smazat `#![allow(unused_imports, unused_variables, unused_mut)]` z vrchu `auth.rs` — maskuje právě tenhle problém, kompilátor by jinak varoval.

- [ ] **Jednotky delay v config neodpovídají použití.** `FaultInjectConfig.min_delay`/`max_delay` komentář v [src/config.rs:18-19](src/config.rs#L18) říká „v mikrosekundách", ale [src/app.rs:134](src/app.rs#L134) je interpretuje jako milisekundy (`Duration::from_millis`). Oprav komentář nebo převod — ať sedí.

- [ ] **`error_rate` nedělá to, co deklaruje.** Původní "fuzzy error" check (`rng.gen_range(..) < error_rate`) je zakomentovaný v [src/fault_inject.rs:154-156](src/fault_inject.rs#L154). Místo něj se chyba spouští jako vedlejší efekt výpočtu zpoždění (`spread = (max-min)*(1+error_rate*2)`, a když `extra > max_delay`, request rovnou skončí chybou — [src/fault_inject.rs:109-138](src/fault_inject.rs#L109)). S defaultní konfigurací (`error_rate=0.1, min=50ms, max=100ms`) to reálně dává chybovost ~16 %, ne 10 %, a není to přímo řízené `error_rate`. Rozhodnout, co je zamýšlené chování, a předělat buď na čistý fuzzy-error check, nebo na jasně dokumentovaný "overflow-based" model.

- [ ] **`timeout` v config je mrtvý.** Validuje se při startu (`InvalidTimeout` při 0, [src/app.rs:125-129](src/app.rs#L125)), ale v `fault_inject.rs` se nikde nepoužívá — per-request timeout se nikdy neuplatní. Buď implementovat, nebo pole/validaci zatím odstranit.

- [ ] **Chyby v YAML configu nejsou vidět.** `init_fault_inject` je `async fn`, i když nic uvnitř není async ([src/app.rs:107](src/app.rs#L107)) — proto warning na nepoužitý `log`. Když je `error_rate`/`min_delay > max_delay`/`timeout=0` špatně, `.with_result()` middleware tiše vypne ([src/app.rs:153-161](src/app.rs#L153)) bez jakéhokoliv logu. Zalogovat chybu přes `log.error(...)` před fallbackem.

- [ ] **`FileConfigLoader::load` panic-uje na čemkoliv nevalidním** — chybějící soubor, špatné YAML, špatný typ hodnoty u klíče, vše přes `.unwrap()` ([src/config.rs:99-100](src/config.rs#L99) a dál). Appka spadne s holým panic trace. Předělat `load()` na `Result<Config, ConfigError>` a vracet smysluplnou chybu.

## 🟡 Konzistence / architektura (k debatě)

- [ ] **`src/otel.rs` je prázdná slupka** (`// TODO - Maybe !`), skutečná OTel inicializace žije v `app.rs::init_tracer()`. Rozhodnout novou strukturu (probereme architekturu zvlášť).
- [ ] **YAML parsing je "type-first" místo "key-first"** ([src/config.rs:105-141](src/config.rs#L105)) — větví se nejdřív podle typu hodnoty, pak podle klíče. `port: "8080"` (string místo čísla) se tiše ignoruje místo chyby. Zvážit přepsání na match-podle-klíče s explicitní konverzí/chybou.
- [ ] Zavádějící komentář `// nested mapping - currently not used` ([src/config.rs:137](src/config.rs#L137)) hned pod kódem, který nested mapping (`fault_inject`) právě zpracovává.
- [ ] `use std::default;` + `impl default::Default for FaultInjectConfig` ([src/config.rs:1](src/config.rs#L1), [:32](src/config.rs#L32)) vs. plain `impl Default for Config` o pár řádků níž ve stejném souboru — sjednotit styl.
- [ ] `main.rs` dumpuje config přes `println!` ([src/main.rs:8](src/main.rs#L8)) místo přes `Logger`, který appka jinak používá — sjednotit, nebo vědomě nechat jako bootstrap-only výjimku s komentářem proč.
- [ ] Duplicitní získávání traceru (`req.data::<SdkTracer>().unwrap()` dvakrát za sebou v [src/fault_inject.rs:89,94](src/fault_inject.rs#L89) a v [src/articles.rs:122-127](src/articles.rs#L122), kde handler už má `tracer: Data<&SdkTracer>` injectnutý).
- [ ] Hromady zakomentovaného starého span/tracing kódu (3 generace pokusů navrstvené na sobě) v `fault_inject.rs` ([src/fault_inject.rs:83-87,120-127,149-156](src/fault_inject.rs#L83)) a `articles.rs` ([src/articles.rs:133-140](src/articles.rs#L133)) — vyčistit, až se ustálí finální otel přístup.

## 🟢 Housekeeping

- [ ] `tests/out.lst` — binární artefakt (65 KB) commitnutý omylem v `82ce0f8`, nepatří do repa — vyřadit/smazat z historie nebo aspoň z aktuálního stromu + doplnit do `.gitignore`.
- [ ] Žádné automatizované testy (`#[test]`/`#[tokio::test]` nikde v `src/`) — `tests/` obsahuje jen ukázkový JSON payload. Aspoň základní unit testy na `config.rs` (parsing) a `fault_inject.rs` (validace rozsahů) by odhalily část bugů výš.

---

*Vygenerováno při review branch `feat/config` před PR do `master`.*

🤖 Generated with [Claude Code](https://claude.com/claude-code)
