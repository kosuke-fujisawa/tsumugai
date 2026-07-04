# CLI 出力方針

tsumugai の CLI は **人間向け出力** と **機械向け出力（JSON / SARIF）** を分けます。

---

## コマンド一覧

```bash
# シナリオの静的検査（v1 記法。ファイルまたはディレクトリ）
tsumugai check <path>
tsumugai check <path> --format json    # 機械向け JSON
tsumugai check <path> --format sarif   # GitHub Code Scanning 向け SARIF 2.1.0
tsumugai check <path> --no-assets      # background / bgm の実在チェックを省略

# シナリオを 1 経路ぶん自動実行して表示（v1 記法、SPEC 5.1）
tsumugai trace scenario.md
tsumugai trace scenario.md --choices 1,3,1     # 選択肢で選ぶ番号を指定して経路を再現
tsumugai trace scenario.md --format json       # 機械向け JSON（--json も同じ）
tsumugai trace scenario.md --no-assets         # 実行前検査のアセットチェックを省略

# シナリオの対話再生（旧記法）
tsumugai play scenario.md
tsumugai play scenario.md --debug
```

## check の検査対象

- **ファイルを指定**: そのファイルと、リンク（選択肢・ジャンプ）で辿れる `.md` を 1 つのプロジェクトとして検査する
- **ディレクトリを指定**: 配下のすべての `.md`（`README.md` を除く）を 1 つのプロジェクトとして検査する

検査ルールは [SPEC.md 6章](../SPEC.md) の Diagnostic ルール表が正です。すべての Diagnostic は「どこが（ファイルと行）・なぜ（説明）・どう直すか（提案）」を含みます（SPEC 6.1「Diagnostic は学習教材である」）。

---

## check：人間向け出力

rustc 風のフォーマットで、入力 Markdown の該当行を引用して表示します。

### 問題なし

```text
✓ 問題は見つかりませんでした。（2 ファイルを検査）
```

- 終了コード: **0**

### 問題あり

```text
error[broken-link]: このファイルに「run-togather」という見出し（##）はありません。よく似た「## run-together」があります。`[一緒に走る](#run-together)` の間違いではありませんか？
  --> scenario/spring_001.md:9
   |
 9 | - [一緒に走る](#run-togather)
   |
   = help: [一緒に走る](#run-together)
   = note: 関連する行: 13

エラー: 1件  警告: 0件（1 ファイルを検査）
```

- `severity[rule_id]: メッセージ` → 位置（`--> ファイル:行`）→ 入力行の引用 → `= help:`（機械的に適用できる書き換え例）→ `= note:`（関連する行）
- 最初のエラーで止まらず、検出できたすべての Diagnostic をまとめて報告する
- 終了コード: エラーあり → **1**、警告のみ → **0**

---

## check：JSON 出力（`--format json`）

```json
{
  "status": "error",
  "files": ["scenario/spring_001.md"],
  "error_count": 1,
  "warning_count": 0,
  "diagnostics": [
    {
      "rule_id": "broken-link",
      "severity": "error",
      "message": "このファイルに「run-togather」という見出し（##）はありません。よく似た「## run-together」があります。`[一緒に走る](#run-together)` の間違いではありませんか？",
      "file": "scenario/spring_001.md",
      "span": { "line": 9 },
      "related_spans": [{ "line": 13 }],
      "suggestion": "[一緒に走る](#run-together)"
    }
  ]
}
```

### スキーマ

```json
{
  "status": "ok" | "error",
  "files": [string],
  "error_count": number,
  "warning_count": number,
  "diagnostics": [
    {
      "rule_id": string,
      "severity": "error" | "warning",
      "message": string,
      "file": string,
      "span": { "line": number } | null,
      "related_spans": [{ "line": number }],
      "suggestion": string | null
    }
  ]
}
```

- `status` は `"ok"` または `"error"`（警告のみなら `"ok"`）
- 入力パスが存在しない・読めない場合も JSON の形式は崩れず、`rule_id: "io-error"` の diagnostic として報告される
- 終了コード: `status == "error"` → **1**、それ以外 → **0**

---

## check：SARIF 出力（`--format sarif`）

GitHub Code Scanning に取り込める SARIF 2.1.0 を出力します。

```json
{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "tsumugai",
          "version": "0.1.0",
          "informationUri": "https://github.com/kosuke-fujisawa/tsumugai",
          "rules": [
            {
              "id": "broken-link",
              "shortDescription": { "text": "選択肢・ジャンプのリンク先が解決できない" },
              "helpUri": "https://github.com/kosuke-fujisawa/tsumugai/blob/main/SPEC.md"
            }
          ]
        }
      },
      "results": [
        {
          "ruleId": "broken-link",
          "level": "error",
          "message": { "text": "…（message に suggestion を併記）…" },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "scenario/spring_001.md" },
                "region": { "startLine": 9 }
              }
            }
          ]
        }
      ]
    }
  ]
}
```

GitHub Actions での利用例:

```yaml
- run: cargo run -- check scenario/ --format sarif > results.sarif || true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif
```

---

## trace：人間向け出力

実行した経路を、入力 Markdown の行番号付きで上から順に表示します（詳細と JSON スキーマは [TRACE.md](TRACE.md)）。

```text
=== Trace: examples/spring/scenario/spring_001.md ===

▶ シーン spring_001「春・出会い」 (examples/spring/scenario/spring_001.md)
      background: ../assets/bg/school_gate.png
      bgm: ../assets/bgm/spring.ogg
     9| 桜の花びらが舞う通学路。いつもと同じ朝のはずだった。
    11| 幼なじみ: おはよう。今日も遅刻しそうだね。
  ── セクション「選択肢」（17 行目）
    19| 選択肢:
          1. [一緒に走る](#run-together)
          2. [諦めて歩く](#walk-together)
          3. [先に行ってもらう](spring_002.md)
        → 1 を選択「一緒に走る」
  ── セクション「run-together」（23 行目）
    25| 幼なじみ: ほら、急ぐよ！
    29| エンディング: childhood_route

結果: エンディング「childhood_route」に到達しました
入力した選択肢: --choices 1
```

- 実行前に check と同じ検査を行い、**error があれば実行せず check とまったく同じ出力**になる（SPEC 6.1）。warning のみの場合は warning を表示してから経路を表示する
- `--choices` の番号が尽きると選択肢一覧を表示して停止し、次に足す番号を案内する
- 終了コード: check エラー・範囲外の選択番号・ステップ上限到達 → **1**、それ以外（入力待ち停止含む） → **0**

## trace：JSON 出力（`--format json`）

check の JSON の上位互換です。`file`（開始シーン）と `trace` が加わり、実行前検査が error のときや入力パスが読めないときも形式は崩れません（`trace` が `null` になる）。

```json
{
  "status": "ok" | "error",
  "file": string,
  "files": [string],
  "error_count": number,
  "warning_count": number,
  "diagnostics": [ /* check と同じ形式 */ ],
  "trace": {
    "steps": [ /* type タグ付きのステップ列。TRACE.md 参照 */ ],
    "end": { "reason": "ending" | "end_of_file" | "awaiting_choice" | "invalid_choice" | "truncated", /* reason ごとの付加情報 */ },
    "choices_requested": [number],
    "choices_used": number
  } | null
}
```

---

## rule_id 一覧（check / trace）

SPEC.md 6章のルール表（error 12種 + warning 12種）が正です。CLI はこれに加えて、記法ではなく環境の問題を表す `io-error`（error。ファイルが存在しない・読めない）を使います。

---

## 終了コードまとめ

| 状況 | 終了コード |
|---|---|
| 問題なし | 0 |
| 警告のみ（エラーなし） | 0 |
| trace が選択肢の入力待ちで停止 | 0 |
| エラーあり（io-error 含む） | 1 |
| trace の選択番号が範囲外 / ステップ上限到達 | 1 |
| 不明なコマンド / 引数不足 | 1 |

---

## 将来の拡張（予定）

```bash
tsumugai routes scenario.md --json    # 全分岐探索・エンディング網羅（#78）
tsumugai fmt scenario.md              # 推測整形（#86）
tsumugai compile scenario.md --target renpy  # Ren'Py 変換（#79）
```

- `compile` / `routes` も trace と同じく、実行前に check と同じ検査を行い、error があれば同じ形式で報告する方針（SPEC 6.1）
