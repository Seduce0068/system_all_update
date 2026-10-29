# system-all-update

Kubuntu 向けのシステム一括更新ツール。references、システムコンポーネント、
アプリケーションを順に更新する。外部クレートに依存しない。

## 要件

- Rust 1.85 以降（edition 2024）
- Debian / Ubuntu 系（apt）
- 任意: rustup, snap, flatpak, fwupd, npm, cargo-update

## ビルドとインストール

```bash
cargo install --path . --root "$HOME/.local" --locked --force
```

`--root` を省略すると `~/.cargo/bin` に配置される。systemd ユニットは
`~/.local/bin/system-all-update` を前提とするため、`--root "$HOME/.local"`
を必ず指定する。`--force` は同一バージョンの再インストールに必要。

バイナリの配置は `cargo install` に統一している。ビルド時に `$HOME` へ
書き込む `build.rs` は持たない。

## 使い方

```bash
system-all-update                    # すべて更新
system-all-update --dry-run          # 実行内容のみ表示
system-all-update --references-only  # references のみ
system-all-update --system-only      # システムのみ
system-all-update --apps-only        # アプリのみ
system-all-update --install-service  # systemd ユニットを導入
system-all-update --help             # ヘルプ
```

## 更新対象

| 区分 | 内容 |
| --- | --- |
| references | `/home/koppe/references` の `git pull` |
| system | rustup, apt, snap, flatpak, fwupd, npm |
| apps | VS Code, `cargo install-update -a` |

## 自動実行

`--install-service` は次のユニットを `/etc/systemd/system/` に導入する。

- `system-all-update-boot.service`: 起動時
- `system-all-update-resume.service`: suspend / hibernate 復帰時

## snap の自動クローズ

`~/.config/system-all-update/auto-close-snaps` に snap 名を1行1件で
列挙する。更新が実行中アプリに妨げられた場合、該当アプリを終了して
1回だけ再試行する。

## 開発時の検証

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
