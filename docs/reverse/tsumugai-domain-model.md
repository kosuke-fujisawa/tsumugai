# tsumugai ドメインモデル（逆生成）

## 分析日時
2025-09-27

## ドメイン概要
tsumugaiのドメインは「ビジュアルノベル台本の構造化表現と逐次実行」です。Markdownで記述された台本を構造化し、決定的な実行結果を生成することが中核となります。

## エンティティ（Entity）

### Scenario
**説明**: 完全なビジュアルノベル台本を表すエンティティ

```rust
pub struct Scenario {
    id: ScenarioId,                    // 一意識別子
    title: String,                     //台本タイトル
    commands: Vec<StoryCommand>,       // 台本コマンド列
    metadata: ScenarioMetadata,        // メタデータ
}
```

**責務**:
- 台本全体の管理
- コマンド列の保持
- ラベル整合性検証
- コマンドアクセス

**不変条件**:
- IDは一意
- ラベル参照の整合性（参照されるラベルは定義済み）
- コマンド列は非破壊的

**主要メソッド**:
```rust
impl Scenario {
    pub fn new(id: ScenarioId, title: String, commands: Vec<StoryCommand>) -> Self
    pub fn commands(&self) -> &[StoryCommand]
    pub fn find_label(&self, label: &LabelName) -> Option<usize>
    pub fn validate_labels(&self) -> Result<(), DomainError>
    pub fn get_command(&self, index: usize) -> Result<&StoryCommand, DomainError>
}
```

### StoryExecution
**説明**: 台本の実行状態を管理するエンティティ

```rust
pub struct StoryExecution {
    scenario: Scenario,           // 実行対象台本
    state: ExecutionState,        // 現在の実行状態
}
```

**責務**:
- 台本実行の状態追跡
- プログラムカウンター管理
- 実行位置の制御
- スナップショット作成・復元

**不変条件**:
- プログラムカウンターは台本範囲内
- 実行状態の整合性
- スナップショット可能な状態

**主要メソッド**:
```rust
impl StoryExecution {
    pub fn new(scenario: Scenario) -> Result<Self, DomainError>
    pub fn current_command(&self) -> Result<&StoryCommand, DomainError>
    pub fn advance_to(&mut self, position: usize) -> Result<(), DomainError>
    pub fn jump_to_label(&mut self, label: &LabelName) -> Result<(), DomainError>
    pub fn create_snapshot(&self) -> ExecutionSnapshot
    pub fn restore_from_snapshot(&mut self, snapshot: ExecutionSnapshot) -> Result<(), DomainError>
}
```

## 値オブジェクト（Value Object）

### ScenarioId
**説明**: 台本の一意識別子

```rust
pub struct ScenarioId(String);
```

**制約**:
- ASCII英数字、アンダースコア、ハイフンのみ
- パス区切り文字禁止
- 空文字列禁止

### LabelName
**説明**: ジャンプ先ラベルの名前

```rust
pub struct LabelName(String);
```

**特性**:
- 文字列ベースの識別
- 順序付け可能（ソート対応）
- 重複検出対応

### SpeakerName
**説明**: セリフの話者名

```rust
pub struct SpeakerName(String);
```

### ResourceId
**説明**: アセットの論理識別子

```rust
pub struct ResourceId(String);
```

**用途**:
- BGM、効果音、画像、動画の論理名
- 物理パスとの分離
- リゾルバーによる解決対象

### VariableName
**説明**: 台本変数の名前

```rust
pub struct VariableName(String);
```

**特性**:
- BTreeMapのキーとして使用
- 順序付け可能

### StoryValue
**説明**: 台本変数の値

```rust
pub enum StoryValue {
    Integer(i32),
    Boolean(bool),
    Text(String),
}
```

**型変換**:
```rust
impl From<i32> for StoryValue { ... }
impl From<bool> for StoryValue { ... }
impl From<String> for StoryValue { ... }
impl From<&str> for StoryValue { ... }
```

### StoryCommand
**説明**: 台本の個別コマンド

```rust
pub enum StoryCommand {
    Label { name: LabelName },
    Jump { label: LabelName },
    Say { speaker: SpeakerName, text: String },
    PlayBgm { resource: ResourceId },
    PlaySe { resource: ResourceId },
    ShowImage { resource: ResourceId },
    PlayMovie { resource: ResourceId },
    Wait { duration_seconds: f32 },
    Branch { choices: Vec<Choice> },
    SetVariable { name: VariableName, value: StoryValue },
    ModifyVariable { name: VariableName, operation: VariableOperation, value: StoryValue },
    JumpIf { variable: VariableName, comparison: ComparisonOperation, value: StoryValue, label: LabelName },
}
```

**設計原則**:
- 不変性（Immutable）
- 構造化された命令表現
- 型安全な操作

### Choice
**説明**: 分岐選択肢

```rust
pub struct Choice {
    text: String,              // 選択肢表示文字列
    target_label: LabelName,   // ジャンプ先ラベル
}
```

### ExecutionState
**説明**: 実行時状態

```rust
pub struct ExecutionState {
    program_counter: usize,                    // 現在実行位置
    variables: VariableStore,                  // 変数ストア
    branch_state: Option<BranchState>,         // 分岐状態
}
```

**変数ストア型定義**:
```rust
pub type VariableStore = BTreeMap<VariableName, StoryValue>;
```

### BranchState
**説明**: 分岐実行状態

```rust
pub struct BranchState {
    choices: Vec<Choice>,   // 選択肢リスト
    emitted: bool,         // 出力済みフラグ
}
```

### 演算系値オブジェクト

#### VariableOperation
```rust
pub enum VariableOperation {
    Add,        // 加算
    Subtract,   // 減算
}
```

#### ComparisonOperation
```rust
pub enum ComparisonOperation {
    Equal,              // ==
    NotEqual,           // !=
    LessThan,           // <
    LessThanOrEqual,    // <=
    GreaterThan,        // >
    GreaterThanOrEqual, // >=
}
```

## ドメインサービス（Domain Service）

### StoryExecutionService
**説明**: 台本実行の複雑なビジネスロジック

```rust
pub struct StoryExecutionService;

impl StoryExecutionService {
    pub fn execute_next_command(&self, execution: &mut StoryExecution) -> Result<ExecutionResult, DomainError>
    pub fn evaluate_jump_condition(&self, execution: &StoryExecution, condition: &JumpCondition) -> bool
    pub fn apply_variable_operation(&self, current: &StoryValue, operation: VariableOperation, operand: &StoryValue) -> Result<StoryValue, DomainError>
}
```

**責務**:
- コマンド実行ロジック
- 条件分岐評価
- 変数演算
- 状態変更の調整

## リポジトリ契約（Repository Interface）

### ScenarioRepository
**説明**: 台本の永続化インターフェース

```rust
#[async_trait]
pub trait ScenarioRepository: Send + Sync {
    async fn save(&self, scenario: &Scenario) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &ScenarioId) -> Result<Option<Scenario>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<Scenario>, RepositoryError>;
    async fn delete(&self, id: &ScenarioId) -> Result<(), RepositoryError>;
}
```

### ExecutionSnapshotRepository
**説明**: 実行スナップショットの永続化

```rust
#[async_trait]
pub trait ExecutionSnapshotRepository: Send + Sync {
    async fn save_snapshot(&self, scenario_id: &ScenarioId, snapshot: &ExecutionSnapshot) -> Result<(), RepositoryError>;
    async fn load_snapshot(&self, scenario_id: &ScenarioId) -> Result<Option<ExecutionSnapshot>, RepositoryError>;
    async fn list_snapshots(&self, scenario_id: &ScenarioId) -> Result<Vec<ExecutionSnapshot>, RepositoryError>;
}
```

## ドメインエラー（Domain Error）

```rust
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("Invalid scenario: {message}")]
    InvalidScenario { message: String },

    #[error("Undefined label '{label}' referenced at line {line}")]
    UndefinedLabel { label: LabelName, line: usize },

    #[error("Duplicate label '{label}' defined at line {line}")]
    DuplicateLabel { label: LabelName, line: usize },

    #[error("Invalid command index {index}, max is {max}")]
    InvalidCommandIndex { index: usize, max: usize },

    #[error("Variable operation error: {message}")]
    VariableOperationError { message: String },

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
}
```

## ドメインイベント（Domain Event）

### 実行ライフサイクルイベント
```rust
pub enum ExecutionEvent {
    ScenarioStarted { scenario_id: ScenarioId },
    CommandExecuted { command_index: usize, command: StoryCommand },
    LabelReached { label: LabelName },
    BranchEntered { choices: Vec<Choice> },
    BranchSelected { choice_index: usize, target_label: LabelName },
    VariableChanged { name: VariableName, old_value: Option<StoryValue>, new_value: StoryValue },
    ScenarioCompleted { scenario_id: ScenarioId },
    ExecutionError { error: DomainError },
}
```

## 集約ルート（Aggregate Root）

### StoryExecution が Aggregate Root
- **管理対象**: ExecutionState, 変数ストア, 分岐状態
- **境界**: 一つの台本実行セッション
- **不変条件**: 実行状態の整合性、プログラムカウンター有効性
- **操作単位**: コマンド実行、状態変更、スナップショット

## ファクトリ（Factory）

### ScenarioFactory
```rust
pub struct ScenarioFactory {
    parser: Box<dyn ScenarioParser>,
    id_generator: Box<dyn IdGenerator>,
}

impl ScenarioFactory {
    pub async fn create_from_markdown(&self, content: &str) -> Result<Scenario, DomainError>
    pub async fn create_from_file(&self, path: &Path) -> Result<Scenario, DomainError>
}
```

## 仕様パターン（Specification Pattern）

### ラベル整合性仕様
```rust
pub struct LabelConsistencySpecification;

impl LabelConsistencySpecification {
    pub fn is_satisfied_by(&self, scenario: &Scenario) -> Result<(), DomainError> {
        scenario.validate_labels()
    }
}
```

### 実行可能性仕様
```rust
pub struct ExecutableSpecification;

impl ExecutableSpecification {
    pub fn is_satisfied_by(&self, scenario: &Scenario) -> Result<(), DomainError> {
        // 実行に必要な条件チェック
        // - 最低1つのコマンド存在
        // - 無限ループの検出
        // - 到達不可能コードの検出
    }
}
```

## ドメインサービス詳細

### 変数演算サービス
```rust
impl StoryExecutionService {
    fn apply_arithmetic(&self, left: i32, op: VariableOperation, right: i32) -> i32 {
        match op {
            VariableOperation::Add => left + right,
            VariableOperation::Subtract => left - right,
        }
    }

    fn evaluate_comparison(&self, left: &StoryValue, op: ComparisonOperation, right: &StoryValue) -> bool {
        match (left, right) {
            (StoryValue::Integer(l), StoryValue::Integer(r)) => {
                match op {
                    ComparisonOperation::Equal => l == r,
                    ComparisonOperation::NotEqual => l != r,
                    ComparisonOperation::LessThan => l < r,
                    ComparisonOperation::LessThanOrEqual => l <= r,
                    ComparisonOperation::GreaterThan => l > r,
                    ComparisonOperation::GreaterThanOrEqual => l >= r,
                }
            }
            // 他の型組み合わせ...
        }
    }
}
```

## ドメインモデルの特徴

### 不変性重視
- 値オブジェクトは全て不変
- エンティティの状態変更は制御されたメソッド経由のみ
- コマンド列は作成後変更不可

### 型安全性
- 文字列の型別定義（LabelName, SpeakerName等）
- 列挙型による明示的な状態表現
- Result型による例外安全性

### 表現力
- ドメインエキスパートが理解可能な語彙
- ビジネスルールの明示的表現
- 台本構造の忠実な表現

このドメインモデルにより、ビジュアルノベル台本の複雑な実行ロジックを安全かつ拡張可能な形で表現しています。