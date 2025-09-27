# tsumugai データフロー図（逆生成）

## 分析日時
2025-09-27

## メインデータフロー

### Markdown台本 → StepResult 変換フロー

```mermaid
flowchart TD
    A[Markdown 台本] --> B[MarkdownScenarioParser]
    B --> C[MarkdownParser]
    C --> D[StoryCommand列]
    D --> E[Scenario エンティティ]
    E --> F[ラベル検証]
    F --> G[StoryExecution]
    G --> H[Application Engine]
    H --> I[step実行]
    I --> J[StepResult JSON]

    subgraph "Domain Layer"
        D
        E
        F
        G
    end

    subgraph "Application Layer"
        H
        I
    end

    subgraph "Infrastructure Layer"
        B
        C
    end
```

### エンジン実行サイクル

```mermaid
sequenceDiagram
    participant User as ユーザー
    participant Engine as Application Engine
    participant Core as Core Engine (Legacy)
    participant State as ExecutionState
    participant Resolver as AssetResolver

    User->>Engine: step()
    Engine->>Core: step()
    Core->>State: get current command
    State-->>Core: StoryCommand

    alt SAY command
        Core->>Core: emit Say directive
        Core->>State: advance PC
    else PLAY_BGM command
        Core->>Resolver: resolve BGM path
        Resolver-->>Core: resolved path
        Core->>Core: emit PlayBgm directive
        Core->>State: advance PC
    else BRANCH command
        Core->>Core: emit Branch directive
        Core->>State: set branch state
        Note over Core: Wait for user choice
    else JUMP command
        Core->>State: jump to label
        State->>State: update PC to label position
    end

    Core->>Engine: return Step result
    Engine->>Engine: convert to API directives
    Engine-->>User: StepResult
```

## 分岐処理フロー

```mermaid
stateDiagram-v2
    [*] --> Normal: 通常実行
    Normal --> BranchWait: BRANCH命令実行
    BranchWait --> Normal: choose(index)呼び出し
    Normal --> Normal: その他の命令
    Normal --> [*]: 実行終了

    state BranchWait {
        [*] --> EmitChoices
        EmitChoices --> WaitChoice: 選択肢をDirectiveとして返す
        WaitChoice --> JumpToLabel: ユーザーが選択
        JumpToLabel --> [*]: ラベルへジャンプ
    }
```

### 分岐選択詳細フロー

```mermaid
flowchart TD
    A[BRANCH命令] --> B[選択肢パース]
    B --> C{選択肢有効?}
    C -->|Yes| D[Branch Directive生成]
    C -->|No| E[ParseError]
    D --> F[Engine状態をWaitBranch]
    F --> G[ユーザー入力待ち]
    G --> H[choose呼び出し]
    H --> I{インデックス有効?}
    I -->|Yes| J[対象ラベル取得]
    I -->|No| K[ApiError]
    J --> L[ジャンプ実行]
    L --> M[通常実行再開]
```

## 変数管理フロー

```mermaid
flowchart LR
    A[SET命令] --> B[Variable名解析]
    B --> C[Value解析]
    C --> D[VariableStore更新]

    E[MODIFY命令] --> F[Variable名解析]
    F --> G[演算子解析]
    G --> H[現在値取得]
    H --> I[演算実行]
    I --> D

    J[JUMP_IF命令] --> K[Variable名解析]
    K --> L[比較値解析]
    L --> M[現在値取得]
    M --> N[比較演算]
    N --> O{条件満足?}
    O -->|Yes| P[ラベルジャンプ]
    O -->|No| Q[次の命令へ]
```

## エラーハンドリングフロー

```mermaid
flowchart TD
    A[エラー発生] --> B{エラー種別}

    B -->|ParseError| C[行/列番号付きエラー]
    C --> D[ApiError::Parse変換]

    B -->|DomainError| E[ビジネスルール違反]
    E --> F{エラータイプ}
    F -->|UndefinedLabel| G[未定義ラベル警告]
    F -->|InvalidCommand| H[無効コマンドエラー]

    B -->|EngineError| I[実行時エラー]
    I --> J[ApiError::Engine変換]

    G --> K[継続実行]
    H --> L[実行停止]
    J --> L
    D --> L

    K --> M[警告ログ出力]
    L --> N[エラーレスポンス]
```

## アセット解決フロー

```mermaid
sequenceDiagram
    participant Cmd as StoryCommand
    participant Engine as Engine
    participant Resolver as AssetResolver
    participant FS as FileSystem

    Cmd->>Engine: PlayBgm{resource: "intro"}
    Engine->>Resolver: resolve_bgm("intro")
    Resolver->>FS: check assets/bgm/intro.mp3
    FS-->>Resolver: file exists
    Resolver-->>Engine: Some(PathBuf)
    Engine->>Engine: emit PlayBgm{path: Some("...")}

    Note over Engine,Resolver: 未解決の場合
    Cmd->>Engine: PlayBgm{resource: "missing"}
    Engine->>Resolver: resolve_bgm("missing")
    Resolver->>FS: check assets/bgm/missing.mp3
    FS-->>Resolver: file not found
    Resolver-->>Engine: None
    Engine->>Engine: emit PlayBgm{path: None}
    Engine->>Engine: log warning, continue execution
```

## 状態管理フロー

```mermaid
stateDiagram-v2
    [*] --> Ready: Engine作成
    Ready --> Executing: step()呼び出し
    Executing --> Ready: 通常命令完了
    Executing --> WaitingUser: SAY/WAIT命令
    Executing --> WaitingBranch: BRANCH命令
    Executing --> Halted: 台本終了

    WaitingUser --> Executing: step()再呼び出し
    WaitingBranch --> Executing: choose()呼び出し

    Halted --> [*]

    state Executing {
        [*] --> ParseCommand
        ParseCommand --> ExecuteCommand
        ExecuteCommand --> EmitDirectives
        EmitDirectives --> UpdateState
        UpdateState --> [*]
    }
```

### ExecutionState 詳細フロー

```mermaid
classDiagram
    class ExecutionState {
        +program_counter: usize
        +variables: BTreeMap
        +branch_state: Option~BranchState~
        +increment_pc()
        +set_variable()
        +get_variable()
    }

    class BranchState {
        +choices: Vec~Choice~
        +emitted: bool
        +mark_emitted()
    }

    ExecutionState --> BranchState
    ExecutionState --> VariableStore

    note for ExecutionState "プログラムカウンター\n変数ストア\n分岐状態を管理"
    note for BranchState "分岐選択肢\n出力済みフラグ"
```

## パフォーマンス特性

### メモリ使用パターン
```mermaid
flowchart LR
    A[Markdown文字列] --> B[Parse一時データ]
    B --> C[Scenario構造体]
    C --> D[ExecutionState]
    D --> E[StepResult]

    B -.->|GC| X[破棄]
    E -.->|Return後| Y[破棄]

    subgraph "永続メモリ"
        C
        D
    end

    subgraph "一時メモリ"
        A
        B
        E
    end
```

### 実行時コスト
- **パース**: O(n) - 台本行数に比例
- **ラベル検索**: O(log n) - BTreeMap使用
- **変数アクセス**: O(log v) - 変数数に比例
- **命令実行**: O(1) - 定数時間

## 同期・非同期処理

```mermaid
flowchart TD
    A[同期API] --> B[Engine::step]
    B --> C[即座にStepResult返却]

    D[非同期拡張点] --> E[ScenarioParser::parse]
    E --> F[ファイルI/O等]
    F --> G[async/await対応]

    H[Asset Resolver] --> I[同期解決]
    I --> J[ファイル存在確認]

    subgraph "現在実装"
        A
        B
        C
        H
        I
        J
    end

    subgraph "将来拡張"
        D
        E
        F
        G
    end
```

## データ整合性保証

```mermaid
flowchart TD
    A[入力検証] --> B[Markdown構文チェック]
    B --> C[ラベル整合性検証]
    C --> D[型安全な実行]
    D --> E[決定的出力]

    F[不変条件] --> G[PC範囲チェック]
    G --> H[変数型整合性]
    H --> I[分岐状態整合性]

    J[ゴールデンテスト] --> K[入出力ペア検証]
    K --> L[決定性保証]
```

この設計により、tsumugaiは「Markdown台本→決定的StepResult」の変換を安全かつ効率的に実行し、各レイヤーの責務を明確に分離しています。