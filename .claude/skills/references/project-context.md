# treedoc プロジェクトコンテキスト

`treedoc` は `tree` コマンドを拡張し、各ファイル/ディレクトリにサイドカーファイル (`.treedoc.yaml`) で付けたコメントを揃えて出力する Rust 製 CLI ツール。学習目的も兼ねた個人プロジェクトで、現在は初期構築段階（Phase 2 完了、Phase 3 着手予定）。

実装プランは `.tmp/treedoc-implementation-plan.md` に Phase 1〜7 で記載されている。新機能や設計判断を行う前に、該当 Phase の章を必ず参照すること。

---

## コードベース調査ガイド

### モジュール構成の把握方法

最初に読むべきファイルは以下:

- `Cargo.toml` — クレート定義、依存関係（`anyhow`, `clap`, `serde`, `serde_yaml`）、edition (2021)
- `mise.toml` — ツールチェイン固定（Rust 1.85）
- `src/main.rs` — 現状コードはここ 1 ファイルに集約（CLI 定義、`Node` 構造体、ツリー走査・描画）
- `.tmp/treedoc-implementation-plan.md` — 全体ロードマップと各 Phase の設計方針
- `.github/workflows/ci.yml` — CI で実行される検証項目

シングルクレート構成。ワークスペースやサブクレートはなし。`src/lib.rs` はまだ存在せず、Phase が進んだ段階で `main.rs` から責務分割していく想定。

### 既存パターンの調査手順

参照すべき既存パターン:

- **CLI 定義**: `src/main.rs` の `Cli` 構造体（`clap::Parser` の derive スタイル）。サブコマンド導入は Phase 5。
- **ツリーモデル**: `Node { name, is_dir, children }`。子要素ソートは「ディレクトリ優先 → 名前昇順」（`sort_children`）。
- **描画ロジック**: `render` / `render_children`。親階層の「最後の子」状態を `Vec<bool>` で引き回し、`├── └── │` を切り替える。
- **エラーハンドリング**: 現状は `std::io::Result`。Phase 3 以降で `anyhow::Result` ベースに移行予定。

新機能を追加する際は、まず実装プラン (`.tmp/treedoc-implementation-plan.md`) の対応 Phase を読み、設計のポイント・ハマりどころを把握してから着手する。

### テスト構成の確認方法

- 種類: ユニットテストと統合テストを分離する方針。**全て `tests/` ディレクトリに集約する**（現状 `src/main.rs` 末尾の `#[cfg(test)] mod tests` は移行過渡期のもの。新しいテストは `tests/` 配下に書く）。
- 実行: `cargo test`（CI と同じ）。
- 外部依存: なし。一時ディレクトリは `std::env::temp_dir()` を使用。Docker 等は不要。

---

## 実装ガイド

### ビルド・フォーマットコマンド

| 目的 | コマンド | 備考 |
|------|---------|------|
| フォーマット | `cargo fmt --all` | 検査のみは `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | warnings は CI でエラー扱い |
| ビルド | `cargo build` | リリースビルドは `cargo build --release` |
| テスト | `cargo test` | 統合テストもまとめて実行される |

CI (`.github/workflows/ci.yml`) は PR 上で `fmt --check` → `clippy -D warnings` → `build` → `test` を順に実行する。ローカルでも push 前に同等チェックを通すこと。

### 言語固有の実装規約

- **Rust エディション**: 2021。
- **ツールチェイン**: `mise.toml` で Rust 1.85 を固定。ローカルでも mise でこのバージョンを使う。
- **ドキュメントコメント (rustdoc) は日本語で書く**。`///` や `//!` のコメント、関数の説明は日本語で記述する。識別子・コミットメッセージは英語のまま。
- **clippy `-D warnings` 前提**で書く。`unwrap()` の濫用や `clone()` 多用は clippy が指摘する場合があるので適宜対応。
- 学習段階ゆえ、過度な抽象化より「動くこと」を優先（実装プラン §進め方の指針 を参照）。

### テスト配置ルール

- **すべて `tests/` ディレクトリに置く**（プロジェクト方針）。
  - 例: `tests/tree_render.rs`, `tests/sidecar_yaml.rs`
- 各テストファイルは独立した統合テストクレートとしてビルドされるので、`treedoc` 内部関数を呼ぶには `pub` での公開、もしくは `src/lib.rs` への切り出しが必要になる。
- 旧来 `src/main.rs` 末尾にあるユニットテストは順次 `tests/` 配下へ移行する。

### 実装順序

実装プラン (`.tmp/treedoc-implementation-plan.md`) の Phase 順に進める:

1. Phase 1 — プロジェクト雛形＋フラット走査（**完了**）
2. Phase 2 — ツリー描画（**完了**）
3. Phase 3 — `.treedoc.yaml` 読み込み＋コメント整列（**現ブランチ `feat/phase3-comments`**）
4. Phase 4 — `--depth` / `.gitignore` / 色付き出力
5. Phase 5 — `init` / `set` / `edit` サブコマンド（MVP 完成点）
6. Phase 6 — 出力フォーマット拡張（markdown など）
7. Phase 7 — 公開準備（cargo-dist、crates.io 公開）

各 Phase は前段の完了条件を満たしてから次に進む。Phase をまたぐ変更は実装プランそのものを更新すること。

### CI に委ねてよい項目

CI でカバーしている検証はそのまま CI に任せてよいが、現状ローカル実行で困難なものは特にない（Docker 等不要）。クロスプラットフォーム挙動の確認のみ将来的に CI（Phase 7 で cargo-dist 導入時）に委ねる。

---

## レビューガイド

### ファイルパス → カテゴリマッピング

| 変更ファイルのパスパターン | 選択されるカテゴリ |
|--------------------------|-------------------|
| `src/main.rs`, `src/lib.rs`, `src/**/*.rs` | code |
| `tests/**/*.rs` | test |
| `Cargo.toml`, `Cargo.lock` | build |
| `.github/workflows/**`, `mise.toml`, `renovate.json` | build |
| `README.md`, `*.md`, `.tmp/treedoc-implementation-plan.md` | docs |
| 構造変更（モジュール分割、`src/lib.rs` 新設等） | architecture |
| ファイル I/O・外部入力に関わる変更 | security |

### カテゴリ別レビュー観点

#### architecture
- `main.rs` から `lib.rs` への責務分割タイミングが適切か（Phase 3 以降で肥大化しがち）。
- モジュール（CLI、ツリー構築、描画、サイドカー読み込み）の責務分離。
- 実装プランの該当 Phase の設計方針から外れていないか。

#### code
- clippy `-D warnings` を通るか。
- `unwrap()` / `expect()` を運用パスで使っていないか（テストは可）。
- `PathBuf` / `Path` の使い分け、`String` / `&str` の所有権判断が妥当か。
- パス区切りの正規化（Windows でも動くようスラッシュ統一、Phase 3 の方針）。

#### test
- 新規テストが `tests/` 配下に置かれているか。
- 一時ディレクトリの作成・削除がリークしないか（テスト失敗時のクリーンアップ）。
- ツリー描画テストは罫線・末尾改行まで含めて期待値を厳密比較しているか。

#### security
- ユーザー入力パスの扱い（シンボリックリンクループ、巨大ディレクトリ）。
- YAML パースエラー時に内部情報を出しすぎないか。
- 将来的に `--exclude <PATTERN>` 等で外部入力を扱う際の妥当性検証。

#### docs
- rustdoc コメントが**日本語**で書かれているか。
- README と実装プランの整合（特に Phase 3 以降で UI が変わる箇所）。
- 公開オプションのヘルプ文（`clap` の `about` / `help`）が分かりやすいか。

#### build
- `Cargo.toml` の依存追加が実装プランの「採用フェーズ」と整合しているか。
- `Cargo.lock` のコミット忘れ・不要な変更がないか。
- CI が落ちないか（fmt / clippy / build / test）。

### セキュリティチェックリスト

| チェック項目 | 結果 | 備考 |
|-------------|:----:|------|
| ユーザー指定パスをそのまま `fs::read_dir` に渡しているが、シンボリックリンクループの考慮があるか | ✅ / ❌ / N/A | |
| 巨大ディレクトリでメモリ・スタックを使い切らないか（再帰深さ） | ✅ / ❌ / N/A | |
| `serde_yaml` のパース失敗で panic せず、ユーザー向けエラーで終了するか | ✅ / ❌ / N/A | Phase 3 以降 |
| 外部コマンド起動（Phase 5 の `$EDITOR`）でコマンドインジェクションが起きないか | ✅ / ❌ / N/A | Phase 5 以降 |
| 隠しファイル / `.git` を意図せず公開出力に含めていないか | ✅ / ❌ / N/A | |

### テストカバレッジマトリクス（テンプレート）

| 対象 | 関数 | ユニット/統合テスト | 備考 |
|------|------|:------------------:|------|
|  |  |  |  |

---

## プランテンプレート補足

### 影響範囲テーブル

| モジュール / ファイル | 影響 | 備考 |
|----------------------|------|------|
|  |  |  |

（現状はシングルクレート・単一ファイルなので、Phase 3 以降にモジュール分割した時点で「CLI / tree / sidecar / render」等の単位で記述する）

### ファイル構成の記述例

```
src/
├── main.rs            # CLI エントリポイント
├── lib.rs             # 公開 API（Phase 3 以降）
├── tree.rs            # Node 構築
└── render.rs          # ツリー描画
tests/
├── tree_render.rs     # 描画の統合テスト
└── sidecar_yaml.rs    # .treedoc.yaml 読み込みテスト
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
| `.claude/skills/references/project-context.md` | パッケージ構成変更・ビルドコマンド変更・レビュー観点変更 |

`CLAUDE.md` および `.claude/rules/` は現時点で未整備（必要になった時点で作成）。

---

## ラベル・ワークフロー規約

### Issue/PR ラベルの prefix

- **Kind 系の prefix は `Kind:` を使う**（例: `Kind: Bug Fix`, `Kind: Feature`, `Kind: Refactoring`, `Kind: Document`）。
- 既存の GitHub Labels には `Type:` prefix のものが残っているが、新規ラベル付けは `Kind:` に統一する。差し替え時は triage で整理する。
- 優先度は `Priority: Low / Medium / High`、クローズ理由は `Close: Duplicate / WontFix / Invalid`。

### コード生成

該当なし。`build.rs` や `go generate` 相当のコード生成は使用していない。
