# tsumugai Markdown シナリオ記法 v1

## 0. このドキュメントについて

本書は、Rust 製 CLI **tsumugai** が解釈・検査・変換する Markdown ベースのノベルゲームシナリオ記法 v1 を定義する。

- README は「tsumugai が何であるか」を説明する
- 本書（SPEC）は「何が正しい文法・意味であるか」を定義する

本書に記載された仕様が **tsumugai における正とする振る舞い**である。仕様と実装がズレた場合、実装が仕様に追従する。

v1 は旧記法（`[SAY speaker=...]` 等の括弧コマンド、`:::choices` 等のフェンスブロック）を**完全に置き換える**。旧記法との互換性はない（→ 11.1）。

## 1. 設計思想

- **一般的な Markdown をそのまま使う**。独自記法は最小限にする
- シナリオは上から順に読める
- GitHub や一般的なエディタのプレビューで、リンクや構造がそのまま機能する
- ライターが自然言語感覚で書ける
- `tsumugai check` で構造ミス・参照切れ・書き間違いを機械的に検出できる
- **エラーは学習教材である**。ユーザーは仕様書を読んでから書くのではなく、書いてみて check の指摘から正しい書き方を学べる（→ 6.1）
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
- `characters.yaml` が見つからない場合は warning（`missing-characters-file`）を 1 件だけ報告し、`undefined-character` は報告しない（未宣言警告の氾濫を防ぐ）
- `characters.yaml` が存在するのに読み込めない・`characters:` マッピングがない場合は error（`invalid-characters-file`）。このときも `undefined-character` は報告しない

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

H2 見出しからアンカー名を導出する規則（GitHub の slug 生成と同一）:

1. 前後の空白を除去し、小文字化する
2. 空白を `-` に置き換える
3. Unicode の文字（Letter）・結合記号（Mark）・数字（Number）・ハイフン・アンダースコア以外を除去する。ひらがな・カタカナ・漢字・アクセント付きラテン文字は保持される

例: `## Run Together` → `#run-together`、`## 選択肢` → `#選択肢`、`## Café` → `#café`

- 導出したアンカー名が空になる見出し（記号のみ等）は check エラー（`empty-anchor`）
- 同一ファイル内でアンカー名が重複した場合は check エラー（`duplicate-anchor`）。GitHub は重複に `-1` サフィックスを付けて許容するが、v1 では意図しない飛び先を防ぐため error とする
- 見出しとして解釈するのは ATX 形式（`#` で始まる行）のみ。setext 形式（テキスト行の直下に `---` や `===` を引く書き方）は見出しとして扱わず、warning（`setext-heading`）を出して本文として解釈する。段落の直後に空行を入れず区切り線のつもりで `---` を書くと Markdown では見出しになってしまう事故を防ぐため
- H1 はアンカーを持たない。H1 へのリンクは `broken-link` とし、分岐先にできるのは H2 だけであることを案内する

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
- 話者名が `characters.yaml` に宣言されていない場合も、warning（`undefined-character`）付きで**セリフとして解釈する**。宣言の有無で解釈は変わらない。宣言済み話者に限定しないのは、話者名の書き間違い（「幼馴染」と「幼なじみ」など）を検出するため

セリフの形をした段落は常にセリフになる。ナレーションの文頭にコロン付きの語を置きたい場合（`URL: https://...` など）は `undefined-character` の warning が出るため、文頭を工夫する。コロンより前に空白を含む段落はセリフ判定の対象にならず、黙ってナレーションとして扱う。

### 4.3 選択肢

**リンクのみを項目とするリスト**は選択肢ブロックとして扱う。実行はここでユーザー入力待ちになる。

```markdown
- [一緒に走る](#run-together)
- [諦めて歩く](#walk-together)
- [先に行ってもらう](spring_002.md)
```

- リンク先は同一ファイル内アンカー（`#anchor`）、別ファイル（`file.md`）、別ファイル内アンカー（`file.md#anchor`）のいずれか
- `file.md` 単体（アンカーなし）の場合は、当該ファイルの front matter 直後（リード部の先頭）から評価を開始する
- リストは箇条書き（`-` / `*` / `+`）と番号付き（`1.`）のどちらでもよい
- ネストしたリストは v1 非対応。warning（`unsupported-element`）とし、選択肢としては解釈しない
- リンク先に URL スキーム（`https:` 等）を含むものは error（`broken-link`）。飛び先にできるのはプロジェクト内の Markdown ファイルとアンカーのみ
- アンカーが %-エンコードされている場合（日本語見出しへのリンクをエディタが `#%E9%81%B8%E6%8A%9E%E8%82%A2` と書き出す等）はデコードしてから解決する
- リンク以外のテキストを含む項目、リンクでない項目が混ざるリストは check エラー（`invalid-choice-item`）
- リンクを 1 つも含まないリストは選択肢と解釈せず、warning（`linkless-list`）で `fmt` による変換を案内する（→ 7.1）
- 空のリンクテキスト（`[](#x)`）は check エラー（`empty-choice-label`）
- リンク先が解決できない場合は check エラー（`broken-link`）: ファイルが存在しない、アンカーに対応する H2 がない、など

### 4.4 ジャンプ

**リンク 1 つだけの段落**は無条件ジャンプとして扱う。選択肢を経由せずにシーンやセクションを移動する。

```markdown
[翌朝へ](spring_002.md)
```

- リンク先の解決規則と check ルールは選択肢と同じ（`broken-link`）

段落の一部として本文中に現れるリンク（インラインリンク）は v1 では意味を持たず、ナレーションの文字列として扱う。書き損じたジャンプの可能性が高いため、warning（`inline-link`）で「ジャンプのつもりならリンクだけの段落にする」ことを案内する。

### 4.5 エンディング

HTML コメント `<!-- ending: <id> -->` はエンディング到達を表す。実行はここで終了する。

```markdown
<!-- ending: childhood_route -->
```

- `<id>` はエンディング識別子。宣言は不要。使える文字は英数字・ハイフン・アンダースコア
- 同じ ending id には複数の箇所から到達してよい（重複宣言という概念はない）
- ending 以外のキーを持つ HTML コメント（`<!-- key: value -->` 形式）は将来の制御情報用に予約する。未知のキーは warning（`unknown-directive`）
- 上記形式に当てはまらない HTML コメントは通常のコメント（メモ）として無視する

### 4.6 その他の Markdown 要素

引用（`>`）・コードブロック・テーブル・画像・ネストしたリストなど、本書で意味を定義していない Markdown 要素は v1 では解釈せず、warning（`unsupported-element`）として報告したうえで無視する（compile 対象に含めない）。空行を挟んだ水平線（`---`）は区切りとして無視し、警告も出さない。

## 5. 実行モデル

1. シーンは front matter 直後から上から下へ順に評価される
2. 選択肢ブロックに到達するとユーザー入力待ちになり、選ばれたリンク先へジャンプする
3. ジャンプ段落に到達すると無条件にリンク先へ移動する
4. `<!-- ending: ... -->` に到達すると実行終了
5. セクション（H2）の見出し自体は実行に影響しない。前のセクション末尾からは次のセクションへ**フォールスルー**する
6. ファイル末尾に到達すると実行終了（暗黙の終了）

フォールスルーは「上から順に読める」を優先した仕様だが、セクションの末尾が **ending・ジャンプ・選択肢リストのいずれでもない**場合、次のセクションへの意図しない合流の可能性が高い。check はこれを warning（`implicit-fallthrough`）として検出する。選択肢リストで終わるセクションは必ずいずれかへジャンプするため対象外。front matter 直後から最初のセクションまでのリード部も対象外とする（リード部から最初のセクションへのフォールスルーは通常の流れ）。

```markdown
## run-together

幼なじみ: ほら、急ぐよ！

<!-- ending: childhood_route -->   ← これがないと次の walk-together に流れ込む（warning）

## walk-together
```

## 6. check が検出する Diagnostic ルール

`tsumugai check` は本仕様の違反を構造化 Diagnostic（`rule_id` / `severity` / `message` / `span` / `suggestion`）として報告する。

check は Markdown ファイルまたはディレクトリを受け取る。ディレクトリの場合は配下のすべての `.md` を 1 つのプロジェクトとして検査し、`duplicate-scene-id` などのファイル横断検査が全体に効く。ただし `README.md`（大文字小文字を区別しない）はプロジェクトの説明文書でありシーンではないため、ディレクトリ走査からは除外する（ファイルとして明示指定した場合や、シーンからリンクされている場合は検査する）。ファイル単体の場合は、そのファイルとリンクで辿れる範囲を検査対象とする。

### 6.1 Diagnostic は学習教材である

想定ユーザーは、本書を読み込んでから書くのではなく、**まず普通の Markdown 感覚で書き、check の指摘を見て正しい書き方を学ぶ**。これを成立させるため、すべての Diagnostic は次の 3 点を必ず含む。

1. **どこが**: ファイル名と行番号（`span`）
2. **なぜ**: 何が仕様に合わないかの平易な説明（`message`）。rule_id や内部用語だけで済ませない
3. **どう直すか**: 修正方法の案内。機械的に適用できる書き換え例を構成できる場合は `suggestion` に入れ（ユーザーが書いた内容をそのまま使った例が望ましい）、構成できない場合も `message` の中で直し方を言葉で説明する。`suggestion` フィールド自体は省略可能なままとする（現行 analyzer の `Issue.suggestion: Option<String>` と整合）

人間向け出力の例:

```text
error[broken-link]: リンク先が見つかりません
  --> scenario/spring_001.md:14
   |
14 | - [一緒に走る](#run-togather)
   |
   = help: このファイルに「run-togather」という見出し（##）はありません。
           よく似た「## run-together」があります。
           `- [一緒に走る](#run-together)` の間違いではありませんか？
```

あわせて、次の動作原則を守る。

- **最初のエラーで止まらない**。ファイル全体を読み、検出できたすべての Diagnostic をまとめて報告する。1 回の実行で学べる量を最大化する
- **想定外の書き方は「拒否」ではなく「案内」する**。解釈できない行に対しては、最も近い正しい記法を提示する（例: 旧記法 `[SAY speaker=X]` には「`X: 本文` と書いてください」を示す）
- 解釈できない箇所があっても、解釈できた部分の検査は続行する
- **check を知らなくても指摘に到達できる**。`compile` `trace` `routes` も実行前に同じ検査を行い、error があれば処理せず同じ形式で報告する。ユーザーはどのコマンドから入っても正しい書き方を学べる

| rule_id | severity | 内容 |
|---|---|---|
| `missing-scene-id` | error | front matter に `id` がない |
| `invalid-frontmatter` | error | front matter の YAML が解析できない、または値が文字列でない |
| `duplicate-scene-id` | error | シーン ID がプロジェクト内で重複 |
| `duplicate-anchor` | error | 同一ファイル内で H2 アンカー名が重複 |
| `empty-anchor` | error | H2 見出しから導出したアンカー名が空になる |
| `invalid-h1` | error | H1 が複数ある、またはファイル先頭（front matter 直後）以外にある |
| `broken-link` | error | 選択肢・ジャンプのリンク先が解決できない |
| `invalid-choice-item` | error | 選択肢リストにリンク以外の項目が混在 |
| `empty-choice-label` | error | 選択肢のリンクテキストが空 |
| `missing-asset` | error | `background` / `bgm` のパスが実在しない（`--no-assets` で省略可） |
| `legacy-command` | error | 旧記法（`[SAY ...]` 等の括弧コマンド、`:::` ブロック）を検出。新記法への書き換え suggestion を付与 |
| `invalid-characters-file` | error | `characters.yaml` が存在するのに読み込めない、または `characters:` マッピングがない（このとき `undefined-character` は報告しない） |
| `undefined-character` | warning | `characters.yaml` に未宣言の話者 |
| `implicit-fallthrough` | warning | セクションの末尾が ending・ジャンプ・選択肢リストのいずれでもない |
| `missing-title` | warning | H1 タイトルがない |
| `linkless-list` | warning | リンクを 1 つも含まないリスト。選択肢のつもりなら `fmt` での変換を案内する（→ 7.1） |
| `inline-link` | warning | 本文中のインラインリンク。ジャンプのつもりならリンクだけの段落にする |
| `setext-heading` | warning | setext 形式の見出し。セクションにするなら `##` を使う |
| `unsupported-element` | warning | v1 で意味を定義していない Markdown 要素（引用・コードブロック・テーブル・画像・ネストしたリスト） |
| `missing-characters-file` | warning | `characters.yaml` が見つからない（このとき `undefined-character` は報告しない） |
| `unreachable-section` | warning | どこからも参照されず、フォールスルーでも到達しないセクション |
| `deep-heading` | warning | H3 以深の見出し |
| `unknown-frontmatter-key` | warning | front matter の未知キー |
| `unknown-directive` | warning | `<!-- key: value -->` 形式の未知キー |

ルールの追加・変更は本書を先に更新する。

## 7. 推測整形（tsumugai fmt）

v1 記法を知らないユーザーが自然に書いたテキストを、よくある書き方のパターンから推測して v1 記法へ整形する。ユーザーは「まず書く → fmt の整形結果を見る → 誤りだけ指摘して直す」という流れで記法を学ばずに始められる。

```bash
tsumugai fmt scenario.md           # 整形結果と変更点一覧を表示する（ファイルは変更しない）
tsumugai fmt scenario.md --write   # 整形結果をファイルに書き戻す
```

動作原則:

- **変換はすべて決定的なルールベース**。同じ入力からは常に同じ出力になる（LLM 等による推測は行わない。自由な文章からの本格的な構成推測は外部 LLM の役割とし、tsumugai はその結果を check で検証する側に立つ）
- **黙って書き換えない**。変更点は 1 件ずつ「どの行を・どの規則で・どう変えたか」を diff 形式で提示する。レビュー対象は変更点だけに絞られる
- **確信が持てない箇所は変換しない**。check と同じ形式の Diagnostic として報告し、判断をユーザーに返す

### 7.1 認識するパターン

| 入力パターン | 変換先 | rule_id |
|---|---|---|
| `名前「本文」`（小説式かぎ括弧セリフ） | `名前: 本文` | `fmt-kagi-dialogue` |
| `名前（本文）`（丸括弧の内心。話者が characters.yaml に宣言済みの場合のみ） | `名前: （本文）` | `fmt-paren-dialogue` |
| リンクのないリスト（`・項目` の全角中黒リストを含む） | 対応する H2 見出しへのリンク付き選択肢リスト。**全項目に一致する見出しが存在する場合のみ**変換し、見つからない項目があれば変換せず warning | `fmt-linkless-choice` |
| front matter がない | ファイル名から `id:` を生成して先頭に補う | `fmt-missing-frontmatter` |
| 旧記法（`legacy-command` で検出されるもの） | 11.1 の対応表に従って v1 記法へ | `fmt-legacy` |

パターンの追加は本書を先に更新する。

## 8. サンプル

仕様を網羅するサンプルは `examples/spring/` にある。

- `characters.yaml`: キャラクター定義
- `scenario/spring_001.md`: セリフ・ナレーション・選択肢（ファイル内 / ファイル間）・リード部から最初のセクションへのフォールスルー・別ファイルへのジャンプ・エンディング
- `scenario/spring_002.md`: 見出しなしの選択肢リスト・ファイル内ジャンプ・複数エンディング

```bash
tsumugai check examples/spring/scenario/spring_001.md
```

## 9. 非対応事項（v1 で意図的に行わないこと）

- 変数・フラグ・条件分岐（旧 `:::flag` / `:::vars` / `:::when` / `[SET]`）— v2 候補。必要性が実シナリオで確認できてから設計する
- インライン制御（旧 `[c]` クリック待ち）— 演出は compile 先の責務
- 描画・UI・音声再生
- 複雑な式評価言語、スクリプト埋め込み
- エンジン依存の演出仕様

## 10. 記法一覧（クイックリファレンス）

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

## 11. 変更方針

本仕様は実装よりも先に変更される。仕様が曖昧な場合は本書を更新してから実装する。

### 11.1 旧記法からの変更（v0 → v1）

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
