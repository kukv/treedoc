# treedoc プロジェクトコンテキスト

`treedoc` は `tree` コマンドを拡張し、各ファイル/ディレクトリにサイドカーファイル (`.treedoc.yaml`) で付けたコメントを整列出力する Rust 製 CLI ツール。学習目的も兼ねた個人プロジェクト。

進捗ステータス: **Phase 3 完了**（サイドカーファイル読み込み・コメント整列、`src/lib.rs` への分離、`tests/` への統合テスト集約まで実装済み）。次は Phase 4（`--depth` / `.gitignore` 対応 / 色付き出力）。各 Phase の詳細は `.tmp/treedoc-implementation-plan.md` を参照。

---

## コードベース調査ガイド

### モジュール構成の把握方法

最初に読むべきファイル:

- `Cargo.toml` — クレート定義、依存関係（`anyhow`, `clap` 4.x derive, `serde`, `serde_yaml`）、edition 2021
- `mise.toml` — Rust 1.85 を固定
- `src/lib.rs` — 公開 API（`Node`, `Comments`, `SIDECAR_FILENAME`, `build`, `render`）。ツリー構築・描画・サイドカー読み込みのロジックはここに集約。
- `src/main.rs` — 薄い CLI エントリポイント（`clap::Parser` で引数を取り、`build` → `Comments::load` → `render` を呼ぶだけ）
- `tests/integration.rs` — 統合テスト（`treedoc` クレートを外部から利用する形でテスト）
- `.tmp/treedoc-implementation-plan.md` — Phase 1〜7 のロードマップと設計方針
- `.github/workflows/ci.yml` — CI 検証項目

シングルクレート、lib + bin 構成。ワークスペースなし。

### 既存パターンの調査手順

参照すべきパターン:

- **CLI 定義**: `src/main.rs` の `Cli` 構造体（`clap::Parser` derive）。サブコマンド導入は Phase 5。
- **ツリーモデル**: `Node { name, is_dir, rel_path, children }`。`rel_path` はサイドカーキーと突き合わせるための相対パス（区切りはスラッシュ統一、ルート自身は空文字列）。
- **サイドカー読み込み**: `Comments::load(root)` がルート直下の `.treedoc.yaml` を読む。存在しなければ空マップで続行、パースエラー時は `anyhow::Context` でファイルパス付きのメッセージを返す。キーは `normalize_key` で末尾スラッシュを除去して正規化。
- **ツリー構築**: `build` が再帰的に `Node` を組み立てる。子要素ソートは「ディレクトリ優先 → 名前昇順」。**ルート直下の `.treedoc.yaml` 自身は出力対象から除外**。
- **描画ロジック**: `render` がまず全行を `Vec<(label, Option<comment>)>` に集めて最大幅を計算し、`COMMENT_MARGIN`（=2）スペース空けて `# コメント` を付ける。罫線は親階層の「最後の子」状態 `Vec<bool>` で `├── └── │` を切り替える。
- **エラーハンドリング**: `main` と `Comments::load` は `anyhow::Result`、`build` / `render` は `std::io::Result`（FS / I/O が責務範囲）。

新機能追加時は実装プラン (`.tmp/treedoc-implementation-plan.md`) の対応 Phase を読んでから着手する。

### テスト構成の確認方法

- **統合テストのみ**、すべて `tests/` ディレクトリに置く。現状は `tests/integration.rs` 一本。新規テストは機能単位でファイル分割してよい（例: `tests/sidecar_yaml.rs`）。
- 実行: `cargo test`（CI と同じ）。
- 外部依存なし。一時ディレクトリは `std::env::temp_dir()` を使う。Docker 等は不要。
- 内部関数を直接テストしたい場合は `src/lib.rs` で `pub` 公開するか、`#[cfg(test)] pub(crate)` を使う。`#[cfg(test)] mod tests` を `lib.rs` 内に書く形式は採用しない。

---

## 実装ガイド

### ビルド・フォーマットコマンド

| 目的 | コマンド | 備考 |
|------|---------|------|
| フォーマット | `cargo fmt --all` | 検査のみは `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | warnings は CI でエラー扱い |
| ビルド | `cargo build` | リリースは `cargo build --release` |
| テスト | `cargo test` | `tests/` 配下の統合テストもまとめて実行される |

CI (`.github/workflows/ci.yml`) は PR 時に `fmt --check` → `clippy -D warnings` → `build` → `test` を順次実行。push 前にローカルで同等チェックを通すこと。

### 言語固有の実装規約

- **Rust エディション**: 2021。
- **ツールチェイン**: `mise.toml` で Rust 1.85 を固定。ローカルでも mise でこのバージョンを使う。
- **rustdoc コメントは日本語で書く**。`///` `//!` の説明文は日本語。識別子・コミットメッセージは英語のまま。
- **clippy `-D warnings` 前提**で書く。`unwrap()` / `expect()` は運用パスでは避け、`?` と `anyhow::Context` を活用する（`Comments::load` の `with_context` パターン参照）。
- パス区切りは内部的にスラッシュで統一する（Windows でも一貫した挙動を狙う方針、Phase 3 で導入）。
- 学習段階ゆえ過度な抽象化より「動くこと」を優先（実装プラン §進め方の指針）。

### テスト配置ルール

- **すべて `tests/` ディレクトリに置く**。
- 各ファイルは独立した統合テストクレートとしてビルドされるので、`treedoc::*` を `pub` 経由で使う。
- `Node` 構築用ヘルパー（`dir_at` / `file_at` 等）は各テストファイル内にローカル定義する慣習（`tests/integration.rs` 参照）。共通化が必要になったら `tests/common/mod.rs` パターンに移行する。

### 実装順序

`.tmp/treedoc-implementation-plan.md` の Phase 順:

1. Phase 1 — プロジェクト雛形・フラット走査（**完了**）
2. Phase 2 — ツリー描画（**完了**）
3. Phase 3 — `.treedoc.yaml` 読み込み・コメント整列・lib/bin 分離（**完了**）
4. Phase 4 — `--depth` / `.gitignore`（`ignore` クレート移行） / 色付き出力（**次**）
5. Phase 5 — `init` / `set` / `edit` サブコマンド（MVP 完成点）
6. Phase 6 — 出力フォーマット拡張（markdown 等）
7. Phase 7 — 公開準備（cargo-dist, crates.io）

各 Phase は前段の完了条件を満たしてから進む。Phase スコープを変える場合は実装プランそのものを更新する。

### CI に委ねてよい項目

CI が `fmt --check` / `clippy` / `build` / `test` をすべてカバー。ローカル実行困難なものは現状なし。クロスプラットフォームバイナリ生成は Phase 7 で cargo-dist に委ねる予定。

---

## レビューガイド

### ファイルパス → カテゴリマッピング

| 変更ファイルのパスパターン | カテゴリ |
|--------------------------|---------|
| `src/lib.rs`, `src/**/*.rs`（lib 配下） | code |
| `src/main.rs` | code |
| `tests/**/*.rs` | test |
| `Cargo.toml`, `Cargo.lock` | build |
| `.github/workflows/**`, `mise.toml`, `renovate.json` | build |
| `README.md`, `*.md`, `.tmp/treedoc-implementation-plan.md` | docs |
| 公開 API シグネチャ変更、モジュール分割、新規 `src/*.rs` モジュール追加 | architecture |
| ファイル I/O・YAML パース・サブプロセス起動などの変更 | security |

### カテゴリ別レビュー観点

#### architecture
- `lib.rs` の公開 API（`Node`, `Comments`, `SIDECAR_FILENAME`, `build`, `render`）の変更が後方互換を壊さないか。
- ファイル分割するなら lib 内のモジュール構成（例: `tree`, `render`, `sidecar`）が責務分離できているか。
- 実装プランの該当 Phase の設計方針からの逸脱がないか。

#### code
- clippy `-D warnings` を通るか。
- `unwrap()` / `expect()` を運用パスで使っていないか。テストでは可。
- `PathBuf` / `Path` の使い分け、`String` / `&str` の所有権が妥当か。
- パス区切り（スラッシュ正規化）と `rel_path` の組み立てが一貫しているか。
- ルート直下の `.treedoc.yaml` 除外条件など、サイドカー特有の特殊扱いを見落としていないか。

#### test
- 新規テストが `tests/` 配下に置かれているか。
- 一時ディレクトリのリーク防止（テスト失敗時のクリーンアップ）。
- ツリー描画テストは罫線・末尾改行・コメント整列のパディングまで含めて期待値を厳密比較しているか。
- サイドカー有無・末尾スラッシュ正規化・存在しないキーなどのエッジケースをカバーしているか。

#### security
- ユーザー指定パスの扱い（シンボリックリンクループ、巨大ディレクトリ、深い再帰）。
- YAML パースエラー時にメッセージが過剰に内部情報を出していないか（現状は `with_context` でファイルパスを含めるのみ）。
- Phase 5 で `$EDITOR` を起動する際のコマンドインジェクション。
- 隠しファイルや `.git` を意図せず公開出力に含めていないか。

#### docs
- rustdoc コメントが**日本語**で書かれているか。
- README と実装プランの整合（特にユーザー向け CLI 仕様）。
- `clap` の `about` / `help` 文言が分かりやすいか。

#### build
- `Cargo.toml` の依存追加が実装プランの「採用フェーズ」と整合しているか。
- `Cargo.lock` のコミット忘れや過剰な変更がないか。
- CI（fmt / clippy / build / test）が落ちないか。

### セキュリティチェックリスト

| チェック項目 | 結果 | 備考 |
|-------------|:----:|------|
| ユーザー指定パスでシンボリックリンクループに陥らないか | ✅ / ❌ / N/A | 現状未対策。Phase 4 の `ignore` 移行時に確認 |
| 巨大/深いディレクトリで再帰がスタックを使い切らないか | ✅ / ❌ / N/A | |
| `serde_yaml` のパース失敗で panic せず、ユーザー向けエラーで終了するか | ✅ / ❌ / N/A | `Comments::load` は `?` で伝播 |
| 外部コマンド起動（Phase 5 の `$EDITOR`）でインジェクションが起きないか | ✅ / ❌ / N/A | Phase 5 以降 |
| 隠しファイル / `.git` を意図せず公開出力に含めていないか | ✅ / ❌ / N/A | Phase 4 で `--all` / `.gitignore` 対応予定 |

### テストカバレッジマトリクス（テンプレート）

| 対象モジュール | 関数 / シナリオ | 統合テスト | 備考 |
|--------------|----------------|:---------:|------|
|  |  |  |  |

---

## プランテンプレート補足

### 影響範囲テーブル

| モジュール / ファイル | 影響 | 備考 |
|----------------------|------|------|
| `src/lib.rs` |  | 公開 API 変更時は要記載 |
| `src/main.rs` |  | CLI 引数・サブコマンド変更 |
| `tests/*.rs` |  | テスト追加・更新 |
| `Cargo.toml` |  | 依存追加・メタデータ |

### ファイル構成の記述例

```
src/
├── lib.rs              # 公開 API（Node, Comments, build, render）
└── main.rs             # CLI エントリポイント
tests/
└── integration.rs      # 統合テスト
```

Phase 4 以降で lib 内をモジュール分割した場合の例:

```
src/
├── lib.rs              # 再エクスポート
├── tree.rs             # Node 構築
├── render.rs           # ツリー描画
└── sidecar.rs          # .treedoc.yaml 読み込み
```

### テスト戦略テーブル

| テスト対象 | 種類 | 配置 | 備考 |
|-----------|------|------|------|
|  | 統合 | `tests/` |  |

### ドキュメント更新対象

| ドキュメント | 更新条件 |
|-------------|---------|
| `README.md` | 公開オプション・使い方の変更 |
| `.tmp/treedoc-implementation-plan.md` | 設計方針・Phase スコープの変更 |
| `Cargo.toml` の `description` / `keywords` 等 | Phase 7 公開準備時 |
| `.claude/skills/references/project-context.md` | パッケージ構成・ビルドコマンド・レビュー観点・Phase 進捗の変更 |

`CLAUDE.md` および `.claude/rules/` は現時点で未整備。

---

## ラベル・ワークフロー規約

### Issue/PR ラベルの prefix

- **Kind 系の prefix は `Kind:` を使う**（例: `Kind: Bug Fix`, `Kind: Feature`, `Kind: Refactoring`, `Kind: Document`）。
- 既存ラベルには `Type:` prefix のものが残るが、新規ラベル付与は `Kind:` に統一する。整理は triage で実施。
- 優先度は `Priority: Low / Medium / High`、クローズ理由は `Close: Duplicate / WontFix / Invalid`。

### コード生成

該当なし。`build.rs` 等のコード生成は使用していない。
