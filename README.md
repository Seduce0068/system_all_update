# system-all-update

Kubuntu 向けのシステム一括更新ツール。references、システムコンポーネント、
アプリケーションを順に更新する。外部クレートに依存しない。

## 要件

- Rust 1.85 以降（edition 2024）
- Debian / Ubuntu 系（apt）または Termux（pkg）
- 任意: rustup, snap, flatpak, fwupd, npm, cargo-update,
  termux-job-scheduler

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

## 実行モデル

デフォルト実行（オプションなし）は次の3レーンを並列で進め、
ファームウェア更新だけを最後に単独実行する。

| レーン | 内容 |
| --- | --- |
| System packages | apt, VS Code (apt/snap), snap |
| User space | rustup, npm, cargo install-update |
| References & flatpak | git pull, flatpak |

- apt と VS Code の apt 更新は dpkg ロックを共有するため同一レーンで直列
- snap は同一レーンの最後に実行し、sudo 認証の競合を避ける
- fwupd は再起動を伴う可能性があるため、他レーン完了後に単独実行
- 各レーンの出力は完了後にまとめて表示する
- `--system-only` / `--apps-only` / `--references-only` は従来どおり直列

## 更新対象

| 区分 | 内容 |
| --- | --- |
| references | `~/references`（Kubuntu は `/home/koppe/references`）の `git pull` |
| system | rustup (stable のみ), apt/pkg, snap, flatpak, fwupd, npm |
| apps | VS Code, `cargo install-update -a`, Node CLI の shebang 修正 |

- `rustup update stable` のみ実行する。nightly などの他ツールチェーンは
  自動更新しない。必要な場合は `rustup update nightly` を手動で実行する

## 自動実行

`--install-service` はプラットフォームに応じて自動実行を設定する。

### Kubuntu

次のユニットを `/etc/systemd/system/` に導入する。

- `system-all-update-boot.service`: 起動時
- `system-all-update-resume.service`: suspend / hibernate 復帰時

### Termux

`termux-job-scheduler` に job ID 1 を登録する。実行間隔は約 24 時間で、
Android の JobScheduler により正確な時刻指定はできない。

- 再起動後も維持: `--persisted true`
- 実行スクリプト: `~/.config/system-all-update/termux-job.sh`
- ログ: `~/.local/share/system-all-update/job.log`
- 登録確認: `termux-job-scheduler --pending`
- 解除: `termux-job-scheduler --cancel --job-id 1`

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
