# TODO — nedokončená/rozpracovaná místa (stav `feat/config`, 2026-10-14)

Sepsáno po průchodu celým `src/` (1015 řádků) před PR do `master`. Řazeno podle závažnosti.

## 🔴 Bugy / bezpečnost — řešit před mergem

- [x] ~~**Auth je úplně vyřazená.**~~ **Hotovo na `claude-polishing`.** `auth.rs` je přepsaný na poem middleware `Auth` (stejný vzor jako `FaultInject`), attachnutý jen na `/api/v1/article` ([src/app.rs](src/app.rs)) — čte `Authorization: Bearer`, validuje přes OIDC a request bez platného tokenu nikdy nedojde k handleru (403 dřív, než se zavolá `post_article`). Starý `Token: FromRequest` extraktor (co vždy vracel prázdný token) i blanketní `#![allow(unused_...)]` jsou pryč.
  - ⚠️ **Behaviorální změna:** dřív byl endpoint fakticky bez ochrany, teď reálně vynucuje OIDC validaci proti `OIDC_URL` (`https://idp.class.syscallx86.com:8443/...`) — pokud tenhle IDP není v dev prostředí dostupný, `POST /api/v1/article` bude vracet 403 i s platným requestem. Zvážit, jestli chceš možnost auth v dev/local vypnout (např. přes config flag, podobně jako `with_result(fault_inject)`).

- [ ] **Jednotky delay v config neodpovídají použití.** `FaultInjectConfig.min_delay`/`max_delay` komentář v [src/config.rs:18-19](src/config.rs#L18) říká „v mikrosekundách", ale [src/app.rs:134](src/app.rs#L134) je interpretuje jako milisekundy (`Duration::from_millis`). Oprav komentář nebo převod — ať sedí.

- [ ] **`error_rate` nedělá to, co deklaruje.** Původní "fuzzy error" check (`rng.gen_range(..) < error_rate`) je zakomentovaný v [src/fault_inject.rs:154-156](src/fault_inject.rs#L154). Místo něj se chyba spouští jako vedlejší efekt výpočtu zpoždění (`spread = (max-min)*(1+error_rate*2)`, a když `extra > max_delay`, request rovnou skončí chybou — [src/fault_inject.rs:109-138](src/fault_inject.rs#L109)). S defaultní konfigurací (`error_rate=0.1, min=50ms, max=100ms`) to reálně dává chybovost ~16 %, ne 10 %, a není to přímo řízené `error_rate`. Rozhodnout, co je zamýšlené chování, a předělat buď na čistý fuzzy-error check, nebo na jasně dokumentovaný "overflow-based" model.

- [ ] **`timeout` v config je mrtvý.** Validuje se při startu (`InvalidTimeout` při 0, [src/app.rs:125-129](src/app.rs#L125)), ale v `fault_inject.rs` se nikde nepoužívá — per-request timeout se nikdy neuplatní. Buď implementovat, nebo pole/validaci zatím odstranit.

- [ ] **Chyby v YAML configu nejsou vidět.** `init_fault_inject` je `async fn`, i když nic uvnitř není async ([src/app.rs:107](src/app.rs#L107)) — proto warning na nepoužitý `log`. Když je `error_rate`/`min_delay > max_delay`/`timeout=0` špatně, `.with_result()` middleware tiše vypne ([src/app.rs:153-161](src/app.rs#L153)) bez jakéhokoliv logu. Zalogovat chybu přes `log.error(...)` před fallbackem.

- [ ] **`FileConfigLoader::load` panic-uje na čemkoliv nevalidním** — chybějící soubor, špatné YAML, špatný typ hodnoty u klíče, vše přes `.unwrap()` ([src/config.rs:99-100](src/config.rs#L99) a dál). Appka spadne s holým panic trace. Předělat `load()` na `Result<Config, ConfigError>` a vracet smysluplnou chybu.

## 🟡 Konzistence / architektura (k debatě)

- [x] ~~**`src/otel.rs` je prázdná slupka**~~ **Hotovo na `claude-polishing`.** Inicializace (propagator, OTLP exporter, global tracing subscriber) přesunutá z `app.rs` do `src/otel.rs::init()` (a modul teď konečně i reálně zapojený v `lib.rs` — dřív tam vůbec nebyl přes `pub mod otel;`, takže se ani nekompiloval). Root span pro request dál dělá poem's vlastní `OpenTelemetryTracing` middleware (`app.rs`, beze změny — to už byl správný "best practice" kus). Ruční OTel `Tracer::span_builder()` volání v `fault_inject.rs` a `articles.rs` jsou pryč — nahrazená čistými `tracing::trace_span!(...).instrument(...)`, které se díky `tracing-opentelemetry` automaticky zavěsí pod request-span (ambient OTel context nastavený middlewarem). Mimochodem to i opravilo skrytý bug: `fault_inject.rs` dřív dělal `span.enter()` a držel guard přes `.await` (`sleep(...)`) — to korumpuje span data, jakmile se requesty prokládají na stejném vlákně; `.instrument()` je async-safe varianta.
- [x] **Nově nalezený a opravený bug: pořadí middlewarů rozbíjelo trace na dva.** `fault_inject` bylo zavěšené KOLEM `OpenTelemetryTracing` (ne naopak), takže `fault_inject.call` běžel dřív, než OTel middleware vůbec založil root span/ambient context — výsledek: každý request generoval **dva nesouvisející traces** (osamocený HTTP root span + osamocený strom `fault_inject.call`→handler). Opraveno prohozením pořadí `.with()` v [src/app.rs](src/app.rs) tak, aby `OpenTelemetryTracing` bylo nejvíc navenek. Ověřeno nově přidaným regression testem [tests/otel_trace_shape.rs](tests/otel_trace_shape.rs) (in-memory OTel exporter, bez potřeby reálného collectoru) — běží i v CI/`cargo test`, žádný extra setup.
- [ ] **YAML parsing je "type-first" místo "key-first"** ([src/config.rs:105-141](src/config.rs#L105)) — větví se nejdřív podle typu hodnoty, pak podle klíče. `port: "8080"` (string místo čísla) se tiše ignoruje místo chyby. Zvážit přepsání na match-podle-klíče s explicitní konverzí/chybou.
- [ ] Zavádějící komentář `// nested mapping - currently not used` ([src/config.rs:137](src/config.rs#L137)) hned pod kódem, který nested mapping (`fault_inject`) právě zpracovává.
- [ ] `use std::default;` + `impl default::Default for FaultInjectConfig` ([src/config.rs:1](src/config.rs#L1), [:32](src/config.rs#L32)) vs. plain `impl Default for Config` o pár řádků níž ve stejném souboru — sjednotit styl.
- [ ] `main.rs` dumpuje config přes `println!` ([src/main.rs:8](src/main.rs#L8)) místo přes `Logger`, který appka jinak používá — sjednotit, nebo vědomě nechat jako bootstrap-only výjimku s komentářem proč.
- [x] ~~Duplicitní získávání traceru~~ a ~~hromady zakomentovaného starého span/tracing kódu~~ — **hotovo na `claude-polishing`** jako součást úklidu výše (`fault_inject.rs`, `articles.rs` nepotřebují `Data<&SdkTracer>`/`req.data::<SdkTracer>()` vůbec, stačí `tracing::trace_span!`).

## 🟢 Housekeeping

- [ ] `tests/out.lst` — binární artefakt (65 KB) commitnutý omylem v `82ce0f8`, nepatří do repa — vyřadit/smazat z historie nebo aspoň z aktuálního stromu + doplnit do `.gitignore`.
- [ ] Žádné automatizované testy (`#[test]`/`#[tokio::test]` nikde v `src/`) — `tests/` obsahuje jen ukázkový JSON payload. Aspoň základní unit testy na `config.rs` (parsing) a `fault_inject.rs` (validace rozsahů) by odhalily část bugů výš.

---

*Vygenerováno při review branch `feat/config` před PR do `master`.*

🤖 Generated with [Claude Code](https://claude.com/claude-code)
