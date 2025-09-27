# tsumugai 実装パターン集（逆生成）

## 分析日時
2025-09-27

## アーキテクチャパターン

### Clean Architecture実装パターン

#### 1. 依存関係逆転の実現
```rust
// Domain Layer - インターフェース定義
pub trait ScenarioRepository: Send + Sync {
    async fn save(&self, scenario: &Scenario) -> Result<(), RepositoryError>;
}

// Infrastructure Layer - 具体実装
pub struct FileScenarioRepository {
    base_path: PathBuf,
}

impl ScenarioRepository for FileScenarioRepository {
    async fn save(&self, scenario: &Scenario) -> Result<(), RepositoryError> {
        // ファイルシステムへの永続化
    }
}

// Application Layer - 依存注入
pub struct ScenarioService {
    repository: Arc<dyn ScenarioRepository>,
}
```

**特徴**:
- Domain がインターフェースを定義
- Infrastructure が実装を提供
- Application が依存注入で組み立て

#### 2. レイヤー境界の実装
```rust
// contracts/mod.rs - 公開契約
pub use application::api::{Directive, NextAction, StepResult, ApiError};

// application/mod.rs - レイヤー内部のみ
pub(crate) mod engine;
pub(crate) mod services;

// domain/mod.rs - ドメイン純度保持
// 外部依存なし、I/O操作なし
```

### DDD（ドメイン駆動設計）パターン

#### 1. エンティティパターン
```rust
pub struct Scenario {
    id: ScenarioId,           // アイデンティティ
    title: String,
    commands: Vec<StoryCommand>,
    metadata: ScenarioMetadata,
}

impl Scenario {
    // ファクトリメソッド
    pub fn new(id: ScenarioId, title: String, commands: Vec<StoryCommand>) -> Self {
        Self {
            id,
            title,
            commands,
            metadata: ScenarioMetadata::default(),
        }
    }

    // ビジネスロジック
    pub fn validate_labels(&self) -> Result<(), DomainError> {
        // ドメインルールの実装
    }
}
```

#### 2. 値オブジェクトパターン
```rust
// マクロによる共通実装
macro_rules! impl_string_wrapper {
    ($type:ident) => {
        impl From<String> for $type { ... }
        impl From<&str> for $type { ... }
        impl std::fmt::Display for $type { ... }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LabelName(String);
impl_string_wrapper!(LabelName);
```

#### 3. ドメインサービスパターン
```rust
pub struct StoryExecutionService;

impl StoryExecutionService {
    // 複数エンティティに跨るビジネスロジック
    pub fn execute_next_command(
        &self,
        execution: &mut StoryExecution
    ) -> Result<ExecutionResult, DomainError> {
        let command = execution.current_command()?;
        match command {
            StoryCommand::JumpIf { variable, comparison, value, label } => {
                if self.evaluate_condition(execution, variable, comparison, value)? {
                    execution.jump_to_label(label)?;
                }
            }
            // 他のコマンド処理...
        }
        Ok(ExecutionResult::Success)
    }
}
```

#### 4. 集約パターン
```rust
pub struct StoryExecution {
    scenario: Scenario,        // 集約ルート
    state: ExecutionState,     // 内部エンティティ
}

impl StoryExecution {
    // 集約境界の制御
    pub fn advance_to(&mut self, position: usize) -> Result<(), DomainError> {
        // 不変条件チェック
        if position > self.scenario.command_count() {
            return Err(DomainError::InvalidCommandIndex { ... });
        }
        self.state.set_program_counter(position);
        Ok(())
    }
}
```

## デザインパターン

### 1. Strategy Pattern - AssetResolver
```rust
pub trait Resolver {
    fn resolve_bgm(&self, logical: &str) -> Option<PathBuf>;
    fn resolve_se(&self, logical: &str) -> Option<PathBuf> { None }
    fn resolve_image(&self, logical: &str) -> Option<PathBuf> { None }
}

// デフォルト戦略
pub struct FileSystemResolver {
    base_path: PathBuf,
}

// カスタム戦略
pub struct NetworkResolver {
    base_url: String,
}

// 戦略の注入
let engine = Engine::from_markdown_with_resolver(
    markdown,
    Box::new(FileSystemResolver::new("assets/"))
)?;
```

### 2. Repository Pattern
```rust
#[async_trait]
pub trait ScenarioRepository: Send + Sync {
    async fn find_by_id(&self, id: &ScenarioId) -> Result<Option<Scenario>, RepositoryError>;
    async fn save(&self, scenario: &Scenario) -> Result<(), RepositoryError>;
}

// メモリ実装
pub struct InMemoryScenarioRepository {
    scenarios: Arc<RwLock<HashMap<ScenarioId, Scenario>>>,
}

// ファイル実装
pub struct FileScenarioRepository {
    base_path: PathBuf,
}
```

### 3. Adapter Pattern - レガシー統合
```rust
// 新アーキテクチャ
pub struct Engine {
    core: CoreEngine,    // レガシーエンジン
}

impl Engine {
    pub fn step(&mut self) -> Result<StepResult, ApiError> {
        let step = self.core.step();           // レガシー呼び出し
        let directives = self.core.take_emitted();

        // 新APIへの変換
        let api_directives = directives
            .into_iter()
            .map(|d| self.convert_directive(d))
            .collect::<Result<Vec<_>, _>>()?;

        // 新形式で返却
        match step {
            Step::Next => Ok(StepResult {
                next: NextAction::Next,
                directives: api_directives,
            }),
            // 他の変換...
        }
    }
}
```

### 4. Factory Pattern
```rust
pub struct ScenarioFactory {
    parser: Box<dyn ScenarioParser>,
    id_generator: Box<dyn IdGenerator>,
}

impl ScenarioFactory {
    pub async fn create_from_markdown(&self, content: &str) -> Result<Scenario, DomainError> {
        let (commands, title) = self.parser.parse(content).await?;
        let id = self.id_generator.generate_id(&title);
        Ok(Scenario::new(id, title, commands))
    }
}
```

### 5. State Pattern - 実行状態管理
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionState {
    program_counter: usize,
    variables: VariableStore,
    branch_state: Option<BranchState>,
}

// 状態遷移メソッド
impl ExecutionState {
    pub fn enter_branch(&mut self, choices: Vec<Choice>) {
        self.branch_state = Some(BranchState::new(choices));
    }

    pub fn exit_branch(&mut self) {
        self.branch_state = None;
    }
}
```

## エラーハンドリングパターン

### 1. 階層化エラー設計
```rust
// ドメインエラー
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("Undefined label '{label}' at line {line}")]
    UndefinedLabel { label: LabelName, line: usize },
}

// パースエラー
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Missing parameter '{param}' for '{command}' at line {line}")]
    MissingParameter { command: String, param: String, line: usize },
}

// 公開APIエラー
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("parse error at {line}:{column}: {message}")]
    Parse { line: usize, column: usize, message: String },
}

// 変換実装
impl From<ParseError> for ApiError {
    fn from(error: ParseError) -> Self {
        match error {
            ParseError::MissingParameter { command, param, line } => {
                ApiError::Parse {
                    line,
                    column: 0,
                    message: format!("Missing parameter '{}' for '{}'", param, command),
                }
            }
        }
    }
}
```

### 2. Result型の一貫使用
```rust
// 全ての操作でResult型を使用
pub fn step(&mut self) -> Result<StepResult, ApiError> { ... }
pub fn choose(&mut self, index: usize) -> Result<(), ApiError> { ... }
pub fn validate_labels(&self) -> Result<(), DomainError> { ... }

// ?演算子による簡潔なエラー処理
impl Engine {
    pub fn step(&mut self) -> Result<StepResult, ApiError> {
        let step = self.core.step();
        let directives = self.core.take_emitted();
        let api_directives = directives
            .into_iter()
            .map(|d| self.convert_directive(d))
            .collect::<Result<Vec<_>, _>>()?;  // エラー自動伝播

        Ok(StepResult { ... })
    }
}
```

## パーサー実装パターン

### 1. 再帰下降パーサー
```rust
impl MarkdownParser {
    fn parse(mut self) -> Result<(Vec<StoryCommand>, String), ParseError> {
        while self.current_line < self.lines.len() {
            self.parse_line()?;
            self.current_line += 1;
        }
        Ok((self.commands, self.extract_title()))
    }

    fn parse_line(&mut self) -> Result<(), ParseError> {
        let line = self.lines[self.current_line].trim();
        if let Some(cmd_str) = self.extract_command(line) {
            let command = self.parse_command(&cmd_str)?;
            self.commands.push(command);
        }
        Ok(())
    }
}
```

### 2. コマンドパーサーパターン
```rust
fn parse_command(&mut self, cmd_str: &str) -> Result<StoryCommand, ParseError> {
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    let command_name = parts[0];
    let params = self.parse_params(&parts[1..])?;

    match command_name {
        "SAY" => self.parse_say_command(&params),
        "BRANCH" => self.parse_branch_command(&params),
        "JUMP" => self.parse_jump_command(&params),
        _ => Err(ParseError::InvalidSyntax { ... }),
    }
}
```

### 3. パラメータ解析パターン
```rust
fn parse_params(&self, parts: &[&str]) -> Result<HashMap<String, String>, ParseError> {
    let mut params = HashMap::new();
    let full_params = parts.join(" ");

    if full_params.contains(',') {
        // カンマ区切り: choice=左へ, choice=右へ
        let param_parts = Self::split_commas_preserving_quotes(&full_params);
        for part in param_parts {
            if let Some(eq_pos) = part.find('=') {
                let key = part[..eq_pos].trim().to_string();
                let value = part[eq_pos + 1..].trim();
                params.insert(key, Self::unquote(value).to_string());
            }
        }
    } else {
        // スペース区切り: speaker=Hero text="Hello"
        // 実装...
    }

    Ok(params)
}
```

## テストパターン

### 1. ゴールデンテストパターン
```rust
#[test]
fn golden_test_simple_scenario() {
    let markdown = include_str!("../fixtures/simple.md");
    let mut engine = Engine::from_markdown(markdown).unwrap();

    let mut results = Vec::new();
    loop {
        let result = engine.step().unwrap();
        results.push(result.clone());
        if result.next == NextAction::Halt {
            break;
        }
    }

    // 決定的出力を期待値と比較
    let expected = include_str!("../golden/simple.json");
    let actual = serde_json::to_string_pretty(&results).unwrap();
    assert_eq!(actual, expected);
}
```

### 2. 境界値テストパターン
```rust
#[test]
fn test_edge_cases() {
    // 空台本
    let empty = "";
    assert!(Engine::from_markdown(empty).is_err());

    // 無効ラベル参照
    let invalid_label = "[JUMP label=undefined]";
    match Engine::from_markdown(invalid_label) {
        Err(ApiError::Parse { line, message, .. }) => {
            assert!(message.contains("undefined"));
            assert_eq!(line, 1);
        }
        _ => panic!("Expected parse error"),
    }
}
```

### 3. モックパターン
```rust
struct MockResolver;
impl Resolver for MockResolver {
    fn resolve_bgm(&self, logical: &str) -> Option<PathBuf> {
        match logical {
            "test_bgm" => Some(PathBuf::from("/mock/test.mp3")),
            _ => None,
        }
    }
}

#[test]
fn test_with_mock_resolver() {
    let markdown = "[PLAY_BGM name=test_bgm]";
    let mut engine = Engine::from_markdown_with_resolver(
        markdown,
        Box::new(MockResolver)
    ).unwrap();

    let result = engine.step().unwrap();
    // モック動作の検証
}
```

## 非同期処理パターン

### 1. async-trait パターン
```rust
#[async_trait]
pub trait ScenarioParser: Send + Sync {
    async fn parse(&self, content: &str) -> Result<Scenario, ParseError>;
}

#[async_trait]
impl ScenarioParser for MarkdownScenarioParser {
    async fn parse(&self, content: &str) -> Result<Scenario, ParseError> {
        // 非同期処理（ファイルI/O等）
        tokio::task::spawn_blocking(move || {
            // CPU集約的な処理をブロッキングタスクで実行
            let parser = MarkdownParser::new(content);
            parser.parse()
        }).await?
    }
}
```

### 2. 同期・非同期分離パターン
```rust
// 同期API（即座の応答）
impl Engine {
    pub fn step(&mut self) -> Result<StepResult, ApiError> { ... }
    pub fn choose(&mut self, index: usize) -> Result<(), ApiError> { ... }
}

// 非同期拡張（重い処理）
impl Engine {
    pub async fn from_file(path: &Path) -> Result<Self, ApiError> {
        let content = tokio::fs::read_to_string(path).await?;
        Self::from_markdown(&content)
    }
}
```

## メモリ効率パターン

### 1. ゼロコピーパターン
```rust
// 文字列の所有権管理
pub fn unquote(s: &str) -> std::borrow::Cow<str> {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        Cow::Borrowed(&s[1..s.len() - 1])  // ゼロコピー
    } else {
        Cow::Borrowed(s)                   // ゼロコピー
    }
}
```

### 2. Interning パターン
```rust
// 文字列の重複排除（概念実装）
pub struct StringInterner {
    strings: HashMap<String, Arc<str>>,
}

impl StringInterner {
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        self.strings
            .entry(s.to_string())
            .or_insert_with(|| Arc::from(s))
            .clone()
    }
}
```

## 設定・拡張パターン

### 1. Builder パターン
```rust
pub struct EngineBuilder {
    resolver: Option<Box<dyn Resolver>>,
    variables: HashMap<String, String>,
}

impl EngineBuilder {
    pub fn new() -> Self { ... }

    pub fn with_resolver(mut self, resolver: Box<dyn Resolver>) -> Self {
        self.resolver = Some(resolver);
        self
    }

    pub fn with_variable(mut self, name: &str, value: &str) -> Self {
        self.variables.insert(name.to_string(), value.to_string());
        self
    }

    pub fn build(self, markdown: &str) -> Result<Engine, ApiError> {
        // 設定を適用してEngine構築
    }
}
```

### 2. Plugin パターン
```rust
pub trait DirectiveProcessor: Send + Sync {
    fn process(&self, directive: &Directive) -> Result<(), ProcessError>;
    fn supported_types(&self) -> Vec<String>;
}

pub struct EngineWithPlugins {
    engine: Engine,
    processors: Vec<Box<dyn DirectiveProcessor>>,
}
```

これらのパターンにより、tsumugaiは保守性・拡張性・テスト容易性を実現しています。