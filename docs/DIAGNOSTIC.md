# Diagnostic — 構造化エラー・警告の設計方針

この文書は、`tsumugai check` の検査結果である構造化 Diagnostic（`scenario::Diagnostic`）の型設計と設計原則を説明します。

- ルールの一覧と定義（何が error / warning か）: **SPEC.md 6章が正**
- CLI の出力形式（人間向け / JSON / SARIF）: **[CLI_OUTPUT.md](CLI_OUTPUT.md) が正**

---

## 1. 位置づけ

`scenario::Diagnostic` が Diagnostic の役割を担います。
`tsumugai check` の結果として返され、LLM・CI・人間の三者が共通して扱える形式です。

大原則は「**Diagnostic は学習教材である**」（SPEC 6.1）です。ユーザーは仕様書を読んでから書くのではなく、まず普通の Markdown 感覚で書き、check の指摘を見て正しい書き方を学びます。

---

## 2. 型定義

```rust
pub struct Diagnostic {
    pub rule_id: &'static str,      // SPEC 6章のルール ID（例: "broken-link"）
    pub severity: Severity,         // Error / Warning
    pub message: String,            // なぜ仕様に合わないかの平易な説明 + 直し方の案内
    pub file: PathBuf,              // 対象ファイル（複数ファイル入力に対応）
    pub span: Option<Span>,         // 主要な位置
    pub related_spans: Vec<Span>,   // 関連位置（重複相手・合流先・同じ話者の他の行など）
    pub suggestion: Option<String>, // 機械的に適用できる書き換え例
}

pub struct Span {
    pub line: usize,    // 1-origin
}
```

---

## 3. 設計原則（SPEC 6.1）

すべての Diagnostic は次の 3 点を含みます。

1. **どこが**: `file` + `span`（行番号）
2. **なぜ**: `message`。rule_id や内部用語だけで済ませない
3. **どう直すか**: 機械的に適用できる書き換え例を構成できる場合は `suggestion` に入れる（ユーザーが書いた内容をそのまま使った例が望ましい）。構成できない場合も `message` の中で直し方を言葉で説明する

あわせて次の動作原則を守ります。

- **最初のエラーで止まらない**。検出できたすべての Diagnostic をまとめて報告する
- **拒否ではなく案内する**。解釈できない書き方には最も近い正しい記法を提示する（旧記法 → 新記法、typo アンカー → 類似見出しの提案など）
- **`rule_id` で LLM・CI がフィルタ・集計できる**ようにする
- 入出力エラー（パスが存在しない等）でも JSON / SARIF の形式を崩さない（CLI レベルの `io-error` として報告）

---

## 4. rule_id 一覧

SPEC.md 6章のルール表（error 12種 + warning 12種）を参照してください。ルールの追加・変更は SPEC を先に更新します。

CLI はこれに加えて、記法ではなく環境の問題を表す `io-error` を使います（CLI_OUTPUT.md 参照）。

---

## 5. 出力例

人間向け出力・JSON・SARIF の実例は [CLI_OUTPUT.md](CLI_OUTPUT.md) を参照してください。

---

## 付録: 旧記法パイプラインの `analyzer::Issue`（撤去予定）

旧記法（v0）向けの `analyzer::Issue`（`rule_id` / `level`（Error / Warning / Info）/ `message` / `span` / `related_spans` / `suggestion`）は、`trace` / `play` が旧 parser に依存する間だけコード上に残っています。CLI の `check` からは使われていません。runtime の v1 移行（#77〜#78）完了後に analyzer ごと撤去します。
