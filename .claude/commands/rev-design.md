# rev-design

## 目的

既存のコードベースから技術設計文書を逆生成する。実装されたアーキテクチャ、データフロー、API仕様、ドメインモデルを分析し、設計書として文書化する。

## 前提条件

- 分析対象のコードベースが存在する
- `docs/reverse/` ディレクトリが存在する（なければ作成）

## 実行内容

1. **アーキテクチャの分析**
   - プロジェクト構造からアーキテクチャパターンを特定
   - レイヤー構成の確認（Clean Architecture等）
   - ドメイン駆動設計（DDD）パターンの分析
   - 依存関係の流れの確認

2. **データフローの抽出**
   - Markdown解析からStepResult生成までの流れ
   - エンジン実行状態の変遷
   - エラーハンドリングの流れ
   - 分岐・ジャンプ処理の流れ

3. **API仕様の抽出**
   - 公開API型定義の抽出（StepResult, Directive, NextAction等）
   - Engine APIの仕様
   - エラー型の定義
   - シリアライゼーション形式

4. **ドメインモデルの抽出**
   - エンティティ（Scenario, StoryExecution等）
   - 値オブジェクト（StoryCommand, LabelName等）
   - ドメインサービス
   - リポジトリ契約

5. **実装パターンの分析**
   - パーサー実装パターン
   - 状態管理パターン
   - エラーハンドリングパターン
   - テスト戦略

6. **ファイルの作成**
   - `docs/reverse/tsumugai-architecture.md` - アーキテクチャ概要
   - `docs/reverse/tsumugai-dataflow.md` - データフロー図
   - `docs/reverse/tsumugai-api-specs.md` - API仕様
   - `docs/reverse/tsumugai-domain-model.md` - ドメインモデル
   - `docs/reverse/tsumugai-patterns.md` - 実装パターン集

## 使用コマンド

```bash
# フル分析（全設計書生成）
/rev-design

# 特定の設計書のみ生成（将来実装）
/rev-design --target architecture
/rev-design --target api
/rev-design --target domain
```

## 出力例

### tsumugai-architecture.md

```markdown
# tsumugai アーキテクチャ設計（逆生成）

## 分析日時
{実行日時}

## システム概要
tsumugaiは、Markdownで記述されたビジュアルノベル台本を決定的なJSON形式のStepResultに変換するRustライブラリです。

### 実装されたアーキテクチャ
- **パターン**: Clean Architecture + DDD（Domain-Driven Design）
- **言語**: Rust 2024 Edition
- **構成**: ライブラリクレート（examples付き）

### 技術スタック

#### Core Dependencies
- **serde**: JSON シリアライゼーション
- **thiserror**: エラー型定義
- **async-trait**: 非同期トレイト
- **tokio**: 非同期ランタイム（ファイルI/O用）
- **md5**: コンテンツハッシュ生成
- **log**: ログ出力

## レイヤー構成

### Clean Architectureレイヤー
```
src/
├── domain/          # ドメインレイヤー（依存なし）
├── application/     # アプリケーションレイヤー
├── infrastructure/ # インフラストラクチャレイヤー
├── contracts/      # 公開契約
└── engine/         # レガシーエンジン（互換性維持）
```

### レイヤー責務分析
- **ドメイン層**: 台本構文とビジネスルール（I/O なし）
- **アプリケーション層**: Engine組み立て、ユースケース実行
- **インフラストラクチャ層**: Markdownパーサー、リポジトリ実装
- **公開契約**: StepResult/Directive/NextAction（安定API）

## 依存方向

```
Domain ← Application ← Infrastructure
  ↑
examples/, hosts/（外側）
```

## デザインパターン

### 発見されたパターン
- **Repository Pattern**: ScenarioRepository trait
- **Service Pattern**: StoryExecutionService
- **Strategy Pattern**: AssetResolver（BGM/画像解決）
- **Dependency Injection**: Engine構築時の依存注入
- **State Pattern**: ExecutionState による状態管理

### DDD パターン
- **Entity**: Scenario, StoryExecution（アイデンティティあり）
- **Value Object**: StoryCommand, LabelName（値による同一性）
- **Domain Service**: StoryExecutionService
- **Repository**: ScenarioRepository抽象化

## 非機能要件の実装状況

### 決定性
- **ゴールデンテスト**: dump.rs による決定的出力検証
- **状態管理**: 変数・分岐状態の完全追跡
- **再現性**: ExecutionSnapshot による状態保存/復元

### エラーハンドリング
- **構造化エラー**: thiserror による型安全エラー
- **位置情報**: 行/列番号付きパースエラー
- **グレースフル処理**: 未定義ラベルでも継続実行

### テスト戦略
- **Unit Tests**: パーサー/エンジン単体テスト
- **Integration Tests**: シナリオ全体実行テスト
- **Golden Tests**: 出力決定性検証
- **Architecture Tests**: レイヤー境界検証
```

このような形で、tsumugai固有のアーキテクチャ特徴を抽出し、実装からドキュメントを逆生成します。

