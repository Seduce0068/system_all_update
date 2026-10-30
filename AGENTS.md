---
type: Playbook
title: "AGENTS.md for system_all_update"
description: "system_all_update 固有の制約とグローバル規約との差分。"
tags:
  - agents
  - system-all-update
  - rust
generated:
  by: opencode
  at: "2026-10-31T00:27:37+09:00"
---

## プロジェクト概要

Kubuntu と Termux 向けのシステム一括更新ツール。references、システム
コンポーネント、アプリケーションを順に更新する。外部クレートに依存せず、
Rust 1.85 以降（edition 2024）を要求する。

## グローバル設定との関係

- 共通規約はグローバル `~/.config/opencode/AGENTS.md` を継承する。
- 本ファイルは本プロジェクト固有の差分だけを記述する。グローバルと
  重複する規約は追加せず、差分がある場合にのみ上書きする。
- グローバル TODO は `~/.config/opencode/todo.md`。複数プロジェクト
  横断のタスクはそこに置き、本プロジェクト固有のタスクは本リポジトリの
  issue やドキュメントで管理する。
- 本ファイルとグローバルが矛盾する場合は、本ファイルの記述を優先する。

## プロジェクト固有の制約

- システムを変更する処理を検証する場合は `--dry-run` を使い、実更新を
  無断で実行しない。
- `--install-service` は systemd ユニットまたは `termux-job-scheduler` に
  書き込むため、実行前に影響を確認する。
- 秘密情報やホスト固有の設定をリポジトリに含めない。

## 検証

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
