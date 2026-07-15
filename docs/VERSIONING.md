# Versioning — 配布・バージョニング契約（tsumugai ⇄ arikoi）

関連: [ARCHITECTURE.md 8章](ARCHITECTURE.md)、[API.md](API.md)、issue #157

## 目的

arikoi（別リポジトリ、Svelte 製 Web ノベルゲーム）が tsumugai CLI を使って StoryBundle JSON を生成する際に、ビルド結果を再現可能にする。本書は次を明確にする。

- arikoi がどの tsumugai を使うべきか
- どの単位でバージョンを固定するか
- StoryBundle JSON の互換性をどう扱うか
- `schemaVersion` をいつ変更するか

## 基本方針（MVP段階）

tsumugai は引き続き Rust 製 CLI として提供する。以下は MVP 段階では採用しない。

- npm パッケージ化・Node バインディング
- crates.io 公開
- GitHub Releases によるプリビルドバイナリ配布
- arikoi からの import 利用

arikoi との境界は、引き続き CLI サブプロセス + JSON（[ARCHITECTURE.md 8章](ARCHITECTURE.md)）とする。

```text
arikoi
  -> tsumugai CLI
  -> StoryBundle JSON
  -> arikoi runtime
```

## 配布方式

MVP では **git tag による固定** を正式な推奨方式とする。

推奨:

```sh
cargo install --git https://github.com/kosuke-fujisawa/tsumugai --tag v0.1.0
```

> **注記**: git tag はまだ作成していない。arikoi が利用可能な最初の安定点で `v0.1.0` を作る（→ リリース運用）。それまでは次の commit SHA 固定を使う。

検証中に限り commit SHA 固定も許容する。

```sh
cargo install --git https://github.com/kosuke-fujisawa/tsumugai --rev <commit-sha>
```

**禁止・非推奨**（tag/rev を指定しない main 追従）:

```sh
cargo install --git https://github.com/kosuke-fujisawa/tsumugai
```

理由: `main` の更新で arikoi のビルド結果が変わり、StoryBundle の内容が再現できなくなる。arikoi の CI が tsumugai 側の変更で突然壊れる可能性もある。

## リリース運用

### 必須

- `Cargo.toml` の `version` を SemVer として扱う（次節）
- arikoi が利用可能な安定点ができたら git tag を作る（`v0.1.0` 形式）

### MVP では不要

- GitHub Release の作成
- macOS / Windows / Linux 向けプリビルドバイナリ配布
- crates.io publish
- インストーラ作成

## SemVer 方針

`Cargo.toml` の package version は CLI 全体のバージョンを表す。

| 変更 | 例 |
|---|---|
| **patch** | バグ修正、diagnostic の文言改善、内部実装改善、後方互換な warning 追加、StoryBundle への非破壊的な補助フィールド追加 |
| **minor** | 新コマンド追加、既存コマンドへの後方互換オプション追加、新しい check rule の追加、StoryBundle への任意フィールド追加、arikoi 側が無視可能な情報の追加 |
| **major** | CLI 引数の破壊的変更、既存コマンドの意味変更、既存 JSON 出力の必須フィールド変更、StoryBundle の破壊的変更、v1 シナリオ記法を壊す変更 |

## StoryBundle `schemaVersion` 方針

`schemaVersion` は tsumugai CLI のバージョン（`Cargo.toml` の `version`）とは**別**に扱う。

```text
tsumugai version        = CLIツールとしてのバージョン（SemVer）
StoryBundle schemaVersion = arikoi runtime が読む JSON 契約のバージョン
```

現行は `{ "schemaVersion": "1" }` を維持する（[API.md 6.5章](API.md)）。

### 上げる条件

- 必須フィールドの追加・既存フィールドの削除
- 既存フィールドの型変更・意味変更
- `BundleStep` の表現変更
- `target`（jump/choice の飛び先）の解決ルール変更
- runtime が既存の StoryBundle を安全に読めなくなる変更

### 上げない条件

- 任意フィールドの追加、arikoi が無視できるメタデータ追加
- diagnostic / warning rule の追加
- エラーメッセージの改善、内部実装の変更
- 出力の決定性を保ったままの整理

## arikoi 側への推奨事項

tsumugai 側では強制できないが、arikoi 側には次を推奨する。

- tsumugai の tag または commit SHA を固定する（`main` を直接追従しない）
- StoryBundle 読み込み時に `schemaVersion` を検査し、未対応の値は fail hard する
- セーブデータに `storySchemaVersion` と `storyBuildId`（[API.md 6.5章](API.md)）を保存する
- `storyBuildId` が一致しないセーブデータはロード不可にする

## 非要件（本書の対象外）

- crates.io 公開・GitHub Releases 整備・プリビルドバイナリ作成
- npm パッケージ化・Node バインディング作成
- arikoi 側スクリプト（`scripts/check-scenario.sh` 等）の実装
- StoryBundle 構造の変更・`schemaVersion` の即時変更

これらは、実際に必要になった時点で別issueとして扱う。
