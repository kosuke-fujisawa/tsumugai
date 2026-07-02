# tsumugai Markdown シナリオ記法 v1

## 0. このドキュメントについて

本書は、Rust 製 CLI **tsumugai** が解釈・検査・変換する Markdown ベースのノベルゲームシナリオ記法 v1 を定義する。

- README は「tsumugai が何であるか」を説明する
- 本書（SPEC）は「何が正しい文法・意味であるか」を定義する

本書に記載された仕様が **tsumugai における正とする振る舞い**である。仕様と実装がズレた場合、実装が仕様に追従する。

v1 は旧記法（`[SAY speaker=...]` 等の括弧コマンド、`:::choices` 等のフェンスブロック）を**完全に置き換える**。旧記法との互換性はない（→ 10.1）。

## 1. 設計思想

- **一般的な Markdown をそのまま使う**。独自記法は最小限にする
- シナリオは上から順に読める
- GitHub や一般的なエディタのプレビューで、リンクや構造がそのまま機能する
- ライターが自然言語感覚で書ける
- `tsumugai check` で構造ミス・参照切れ・書き間違いを機械的に検出できる
- LLM が安全に解析・生成・変換できる

tsumugai は演出エンジンではない。シナリオの解釈・検査・変換のみを責務とする。

## 2. プロジェクト構成

シナリオプロジェクトは次のファイルで構成される。

```text
project/
├── characters.yaml      # キャラクター定義（プロジェクトに 1 つ）
├── scenario/
│   ├── spring_001.md    # シーン（1 ファイル = 1 シーン）
│   └── spring_002.md
└── assets/
    ├── bg/school_gate.png
    └── bgm/spring.ogg
```

- ディレクトリ名・配置は自由。tsumugai は引数に渡された Markdown ファイルと、そこから相対パスで参照されるファイルだけを読む
- `characters.yaml` はシナリオファイルと同じディレクトリ、またはその祖先ディレクトリに置く。最も近いものが使われる

### 2.1 characters.yaml

登場キャラクター（話者）を事前宣言する。

```yaml
characters:
  幼なじみ:
    color: "#ff9999"
  主人公: {}
```

- キーが話者名。セリフの `名前:` と完全一致で照合する
- 値は任意のメタデータ（`color` など）。tsumugai は中身を解釈せず、compile 先（Ren'Py 等）に引き渡す
- 宣言されていない話者のセリフは check で warning になる（`undefined-character`）

## 3. シーンファイル

1 つの Markdown ファイルが 1 つのシーンである。

### 3.1 Front Matter

ファイル先頭に YAML Front Matter を書く。

```markdown
---
id: spring_001
background: ../assets/bg/school_gate.png
bgm: ../assets/bgm/spring.ogg
---
```

| キー | 必須 | 意味 |
|---|---|---|
| `id` | ✅ | シーン ID。プロジェクト内で一意 |
| `background` | – | 背景画像。ファイルからの相対パス |
| `bgm` | – | BGM。ファイルからの相対パス |

- `id` がない、または重複している場合は check エラー（`missing-scene-id` / `duplicate-scene-id`）
- `background` / `bgm` のパスが実在しない場合は check エラー（`missing-asset`）。`--no-assets` で存在チェックを省略できる
- 未知のキーは warning（`unknown-frontmatter-key`）とし、将来の拡張余地とする

### 3.2 見出し

| レベル | 意味 |
|---|---|
| `#`（H1） | シーンの表示用タイトル。ファイル先頭に 1 つだけ書く。アンカーにはならない |
| `##`（H2） | セクション。分岐先・ジャンプ先として参照できる |
| `###` 以深 | v1 では未対応。check で warning（`deep-heading`）とし、本文としては無視する |

H2 見出しからアンカー名を導出する規則（GitHub 互換）:

1. 前後の空白を除去し、小文字化する
2. 空白を `-` に置き換える
3. 英数字・ハイフン・アンダースコア・CJK 文字以外を除去する

例: `## Run Together` → `#run-together`、`## 選択肢` → `#選択肢`

同一ファイル内でアンカー名が重複した場合は check エラー（`duplicate-anchor`）。

## 4. 本文要素

### 4.1 ナレーション

通常の段落はナレーションとして扱う。

```markdown
桜の花びらが舞う通学路。いつもと同じ朝のはずだった。
```

### 4.2 セリフ

`名前: 本文` 形式の段落はセリフとして扱う。

```markdown
幼なじみ: おはよう。今日も遅刻しそうだね。

主人公: まだ間に合うよ。
```

- 区切りは半角コロン `:` または全角コロン `：`。直後の空白は無視する
- 話者名は段落の先頭から最初のコロンまで。空白を含む名前は認めない
- 1 段落 = 1 発話。段落内の改行は同一発話の継続として扱う
- 話者名が `characters.yaml` に宣言されていない場合は warning（`undefined-character`）。セリフとしての解釈は維持する

ナレーション本文にコロンを含めたい場合（`URL: https://...` など）、先頭語が宣言済みキャラクターと一致しなければ通常はナレーション扱いだが、`undefined-character` の warning が出る。回避するには文頭を工夫するか、キャラクター宣言を見直す。

### 4.3 選択肢

**リンクのみを項目とするリスト**は選択肢ブロックとして扱う。実行はここでユーザー入力待ちになる。

```markdown
- [一緒に走る](#run-together)
- [諦めて歩く](#walk-together)
- [先に行ってもらう](spring_002.md)
```

- リンク先は同一ファイル内アンカー（`#anchor`）、別ファイル（`file.md`）、別ファイル内アンカー（`file.md#anchor`）のいずれか
- リンク以外のテキストを含む項目、リンクでない項目が混ざるリストは check エラー（`invalid-choice-item`）
- 空のリンクテキスト（`[](#x)`）は check エラー（`empty-choice-label`）
- リンク先が解決できない場合は check エラー（`broken-link`）: ファイルが存在しない、アンカーに対応する H2 がない、など

### 4.4 ジャンプ

**リンク 1 つだけの段落**は無条件ジャンプとして扱う。選択肢を経由せずにシーンやセクションを移動する。

```markdown
[翌朝へ](spring_002.md)
```

- リンク先の解決規則と check ルールは選択肢と同じ（`broken-link`）

### 4.5 エンディング

HTML コメント `<!-- ending: <id> -->` はエンディング到達を表す。実行はここで終了する。

```markdown
<!-- ending: childhood_route -->
```

- `<id>` はエンディング識別子。宣言は不要
- ending 以外のキーを持つ HTML コメント（`<!-- key: value -->` 形式）は将来の制御情報用に予約する。未知のキーは warning（`unknown-directive`）
- 上記形式に当てはまらない HTML コメントは通常のコメント（メモ）として無視する

## 5. 実行モデル

1. シーンは front matter 直後から上から下へ順に評価される
2. 選択肢ブロックに到達するとユーザー入力待ちになり、選ばれたリンク先へジャンプする
3. ジャンプ段落に到達すると無条件にリンク先へ移動する
4. `<!-- ending: ... -->` に到達すると実行終了
5. セクション（H2）の見出し自体は実行に影響しない。前のセクション末尾からは次のセクションへ**フォールスルー**する
6. ファイル末尾に到達すると実行終了（暗黙の終了）

フォールスルーは「上から順に読める」を優先した仕様だが、分岐先セクションの末尾が ending でもジャンプでもない場合、意図しない合流の可能性が高い。check はこれを warning（`implicit-fallthrough`）として検出する。

```markdown
## run-together

幼なじみ: ほら、急ぐよ！

<!-- ending: childhood_route -->   ← これがないと次の walk-together に流れ込む（warning）

## walk-together
```

## 6. check が検出する Diagnostic ルール

`tsumugai check` は本仕様の違反を構造化 Diagnostic（`rule_id` / `severity` / `message` / `span` / `suggestion`）として報告する。

| rule_id | severity | 内容 |
|---|---|---|
| `missing-scene-id` | error | front matter に `id` がない |
| `duplicate-scene-id` | error | シーン ID がプロジェクト内で重複 |
| `duplicate-anchor` | error | 同一ファイル内で H2 アンカー名が重複 |
| `broken-link` | error | 選択肢・ジャンプのリンク先が解決できない |
| `invalid-choice-item` | error | 選択肢リストにリンク以外の項目が混在 |
| `empty-choice-label` | error | 選択肢のリンクテキストが空 |
| `missing-asset` | error | `background` / `bgm` のパスが実在しない（`--no-assets` で省略可） |
| `legacy-command` | error | 旧記法（`[SAY ...]` 等の括弧コマンド、`:::` ブロック）を検出。新記法への書き換え suggestion を付与 |
| `undefined-character` | warning | `characters.yaml` に未宣言の話者 |
| `implicit-fallthrough` | warning | 分岐先セクションの末尾に ending もジャンプもない |
| `unreachable-section` | warning | どこからも参照されず、フォールスルーでも到達しないセクション |
| `deep-heading` | warning | H3 以深の見出し |
| `unknown-frontmatter-key` | warning | front matter の未知キー |
| `unknown-directive` | warning | `<!-- key: value -->` 形式の未知キー |

ルールの追加・変更は本書を先に更新する。

## 7. サンプル

仕様を網羅するサンプルは `examples/spring/` にある。

- `characters.yaml`: キャラクター定義
- `scenario/spring_001.md`: セリフ・ナレーション・選択肢（ファイル内 / ファイル間）・エンディング
- `scenario/spring_002.md`: ジャンプ・フォールスルー・複数エンディング

```bash
tsumugai check examples/spring/scenario/spring_001.md
```

## 8. 非対応事項（v1 で意図的に行わないこと）

- 変数・フラグ・条件分岐（旧 `:::flag` / `:::vars` / `:::when` / `[SET]`）— v2 候補。必要性が実シナリオで確認できてから設計する
- インライン制御（旧 `[c]` クリック待ち）— 演出は compile 先の責務
- 描画・UI・音声再生
- 複雑な式評価言語、スクリプト埋め込み
- エンジン依存の演出仕様

## 9. 記法一覧（クイックリファレンス）

| 書くもの | 記法 |
|---|---|
| シーン ID・背景・BGM | YAML Front Matter |
| シーンタイトル | `# タイトル` |
| 分岐先セクション | `## セクション名` |
| ナレーション | 通常の段落 |
| セリフ | `名前: 本文` |
| 選択肢 | `- [ラベル](#anchor)` のリンクだけのリスト |
| ジャンプ | `[ラベル](file.md#anchor)` だけの段落 |
| エンディング | `<!-- ending: id -->` |
| メモ | 上記形式以外の HTML コメント |

## 10. 変更方針

本仕様は実装よりも先に変更される。仕様が曖昧な場合は本書を更新してから実装する。

### 10.1 旧記法からの変更（v0 → v1）

| 旧（v0） | 新（v1） |
|---|---|
| `# scene: name` | front matter `id:` + `# 表示タイトル` |
| `[SAY speaker=X]` + 本文 | `X: 本文` |
| `[BRANCH choice=a choice=b]` / `:::choices` | リンクのみのリスト |
| `[LABEL name=x]` / `:::route x` | `## x` |
| `[JUMP label=x]` | `[ラベル](#x)` だけの段落 |
| `[SHOW_IMAGE file=x]` / `[PLAY_MUSIC file=x]` | front matter `background:` / `bgm:` |
| `[WAIT 1.0s]` / `[c]` | 廃止（演出は compile 先の責務） |
| `:::flag` / `:::vars` / `:::when` / `[SET ...]` | v1 では非対応（v2 候補） |

旧記法は v1 パーサーでは解析されず、check が `legacy-command` エラーとして検出する。
