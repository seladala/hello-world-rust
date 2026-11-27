# Как внести изменения

## Подготовка

Установите [rustup](https://rustup.rs/) и откройте корень репозитория. `rust-toolchain.toml` выбирает Rust 1.87.0 вместе с rustfmt и Clippy.

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

Если редактируете `src/`, сначала отформатируйте код, затем обновите манифест и повторите тесты:

```bash
cargo fmt --all
cargo run --locked --example update_checksums
cargo test --locked --all-targets
```

В коммит входят изменённые исходники и `src/core/security/checksums.sha256`. Интеграционный тест требует ровно 67 000 строк в `src/main.rs`. При штатном запуске stdout содержит только `hello world` с переводом строки.

## Pull request

Опишите изменение и команды проверки. При обновлении замеров приложите JSON с сырыми отсчётами, версией компилятора и окружением. Сравнивайте запуски на одинаковых машинах.

## Выпуск

После зелёного CI синхронизируйте версию в `Cargo.toml`, `Cargo.lock`, `src/core/mod.rs` и [истории изменений](CHANGELOG.md). При изменении исходников обновите манифест. Поставьте аннотированный тег `vMAJOR.MINOR.PATCH` на опубликованный коммит и отправьте тег. Workflow Release проверит соответствие версии, соберёт и протестирует архивы, затем опубликует их с файлами SHA-256. Повторный запуск доступен через Release → Run workflow с существующим тегом.
