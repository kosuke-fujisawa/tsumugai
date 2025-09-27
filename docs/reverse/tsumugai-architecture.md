# tsumugai アーキテクチャ設計（逆生成）

## 分析日時
2025-09-27

## システム概要
tsumugaiは、Markdownで記述されたビジュアルノベル台本を決定的なJSON形式のStepResultに変換するRustライブラリです。

### 実装されたアーキテクチャ
- **パターン**: Clean Architecture + DDD（Domain-Driven Design）
- **言語**: Rust 2024 Edition
- **構成**: ライブラリクレート（examples付き）

### 技術スタック

#### Core Dependencies
- **serde**: JSON シリアライゼーション・デシリアライゼーション
- **thiserror**: 構造化エラー型定義
- **async-trait**: 非同期トレイト対応
- **tokio**: 非同期ランタイム（ファイルI/O・fs・rt・macros）
- **md5**: コンテンツハッシュ生成
- **log**: ログ出力インターフェース

## レイヤー構成

### Clean Architectureレイヤー
```
src/
├── domain/          # ドメインレイヤー（依存なし）
│   ├── entities.rs      # エンティティ（Scenario, StoryExecution）
│   ├── value_objects.rs # 値オブジェクト（StoryCommand, LabelName等）
│   ├── services.rs      # ドメインサービス
│   ├── repositories.rs  # リポジトリ抽象化
│   └── errors.rs        # ドメインエラー
├── application/     # アプリケーションレイヤー
│   ├── engine.rs        # 高レベルEngine API
│   ├── api.rs           # 公開API型定義
│   ├── services.rs      # アプリケーションサービス
│   ├── use_cases.rs     # ユースケース実装
│   └── dependency_injection.rs # DI設定
├── infrastructure/ # インフラストラクチャレイヤー
│   ├── parsing.rs       # Markdownパーサー
│   ├── repositories.rs  # リポジトリ実装
│   ├── md_parser.rs     # レガシーパーサーアダプター
│   └── resource_resolution.rs # アセット解決
├── contracts/      # 公開契約
├── engine/         # レガシーエンジン（互換性維持）
└── [legacy modules] # 後方互換性モジュール
```

### レイヤー責務分析
- **ドメイン層**: 台本構文とビジネスルール（I/O なし、純粋な論理）
- **アプリケーション層**: Engine組み立て、ユースケース実行、ドメインの調整
- **インフラストラクチャ層**: Markdownパーサー、ファイルI/O、リポジトリ実装
- **公開契約**: StepResult/Directive/NextAction（安定API、破壊変更禁止）

## 依存方向

```
Domain ← Application ← Infrastructure
  ↑            ↑
  |            |
  └── examples/, hosts/（外側）
```

## デザインパターン

### 発見されたパターン
- **Repository Pattern**: `ScenarioRepository` trait による永続化抽象化
- **Service Pattern**: `StoryExecutionService` による複雑なビジネスロジック
- **Strategy Pattern**: `AssetResolver` による BGM/画像リソース解決戦略
- **Dependency Injection**: Engine構築時の依存注入（resolver等）
- **State Pattern**: `ExecutionState` による実行状態管理
- **Adapter Pattern**: レガシーパーサーの新アーキテクチャ適応

### DDD パターン
- **Entity**: `Scenario`, `StoryExecution`（アイデンティティと生命周期あり）
- **Value Object**: `StoryCommand`, `LabelName`, `SpeakerName`（値による同一性）
- **Domain Service**: `StoryExecutionService`（エンティティを超えたビジネスロジック）
- **Repository**: `ScenarioRepository` による永続化抽象化
- **Aggregate**: `StoryExecution` が `ExecutionState` を管理

## アーキテクチャ特性

### 決定性の保証
- **ゴールデンテスト**: `examples/dump.rs` による決定的出力検証
- **状態管理**: 変数・分岐状態の完全追跡（`ExecutionState`）
- **再現性**: `ExecutionSnapshot` による状態保存/復元機能
- **純粋関数**: ドメイン層のビジネスロジックは副作用なし

### エラーハンドリング
- **構造化エラー**: `thiserror` による型安全な階層化エラー
- **位置情報**: 行/列番号付きパースエラー（`ApiError::Parse`）
- **グレースフル処理**: 未定義ラベル等でも継続実行可能
- **エラー変換**: 各レイヤー間のエラー型変換（From実装）

### テスト戦略
- **Unit Tests**: パーサー/エンジン単体テスト（`tests/unit_*.rs`）
- **Integration Tests**: シナリオ全体実行テスト（`tests/integration_tests.rs`）
- **Golden Tests**: 出力決定性検証（`tests/golden_tests.rs`）
- **Architecture Tests**: レイヤー境界検証（`tests/clean_architecture_*.rs`）
- **DDD Tests**: ドメイン不変条件テスト（`tests/ddd_*.rs`）

## 公開API設計

### 安定契約
```rust
// 公開API型（破壊変更禁止）
pub enum NextAction { Next, WaitUser, WaitBranch, Halt }
pub enum Directive { Say, PlayBgm, Wait, Branch, ... }
pub struct StepResult { next: NextAction, directives: Vec<Directive> }
pub enum ApiError { Parse, Invalid, Engine, Io }
```

### Engine API
```rust
impl Engine {
    pub fn from_markdown(src: &str) -> Result<Self, ApiError>
    pub fn from_markdown_with_resolver(src: &str, resolver: Box<dyn Resolver>) -> Result<Self, ApiError>
    pub fn step(&mut self) -> Result<StepResult, ApiError>
    pub fn choose(&mut self, index: usize) -> Result<(), ApiError>
    pub fn get_var(&self, name: &str) -> Option<String>
    pub fn set_var(&mut self, name: &str, value: &str)
}
```

## 非機能要件の実装状況

### パフォーマンス
- **ゼロコピー**: 可能な限り文字列のコピーを避ける設計
- **遅延評価**: アセット解決の遅延実行
- **メモリ効率**: BTreeMap による効率的な変数ストレージ
- **パースコスト**: 一度のパースで全体を IR に変換

### セキュリティ
- **入力検証**: Markdown 構文の厳密な検証
- **リソース制限**: 無限ループ・スタックオーバーフロー対策
- **アセット検証**: リソースパス検証（ディレクトリトラバーサル対策）

### 拡張性
- **プラグアーキテクチャ**: Resolver trait による拡張点
- **パーサー拡張**: ScenarioParser trait による形式追加
- **レガシー互換**: 段階的移行のための適応層

## ドメインモデルの特徴

### 不変条件
- ラベル参照の整合性（未定義ラベル検出）
- 実行状態の整合性（プログラムカウンターの範囲）
- 変数型の一貫性（型安全な変数操作）

### ビジネスルール
- 分岐選択は1つのラベルのみ対象
- WAIT命令は正の秒数のみ
- 台本実行は決定的（同一入力→同一出力）

## 設計上の制約・判断

### 破壊変更禁止
- `StepResult/Directive/NextAction` は公開契約のため変更不可
- レガシーAPI は `@deprecated` で段階的廃止

### TDD優先
- 失敗テストから実装開始
- 単位：「1行の解釈→StepResult」
- 種別：ユニット・分岐DFS・ゴールデン・Lint

### DRY原則
- 重複は3回目で抽象化
- 「整える→直す」（構造変更と挙動変更は別コミット）

## 今後の拡張方向

### 計画中の機能
- CUI実行サンプル（`examples/cui_runner/`）
- 参照GUI（`hosts/tauri_svelte/`）最小実装
- スキーマ定義（`schemas/stepresult.schema.json`）

### 禁止事項
- 公開契約の破壊変更
- GUI依存の重い機能追加
- 大容量アセットの同梱
- CI の GUI ビルド依存