# サンプル: fmt（推測整形）

`tsumugai fmt` が SPEC 7.1 の 5 パターンをどう変換するか、レビューできるようにした例。
`before.md` は v1 記法を知らずに自然に書いたテキスト、`after.md` は
`tsumugai fmt examples/fmt/before.md --write` を実行した結果。

```text
fmt/
├── characters.yaml   # 「先生」を宣言（fmt-paren-dialogue の判定に使う）
├── before.md          # 整形前（自然に書いたテキスト）
└── after.md           # 整形後（tsumugai fmt --write の実行結果）
```

## 実行例

```bash
tsumugai fmt examples/fmt/before.md
```

```text
error[legacy-command]: `[SHOW_IMAGE ...]` は旧記法（v0）の括弧コマンドです。v1 では使えません
  --> examples/fmt/before.md:7
   |
 7 | [SHOW_IMAGE name=classroom_evening]
   |
   = help: 背景・BGM は front matter に書いてください（例: `background: ../assets/bg/xxx.png`）

=== fmt: examples/fmt/before.md ===
[fmt-missing-frontmatter] 1行目
+ ---
+ id: before
+ ---

[fmt-kagi-dialogue] 3行目
- あゆみ「今日は疲れたね。」
+ あゆみ: 今日は疲れたね。

[fmt-paren-dialogue] 5行目
- 先生（少し驚いた顔をする）
+ 先生: （少し驚いた顔をする）

[fmt-linkless-choice] 9行目
- ・走って帰る
- ・歩いて帰る
+ - [走って帰る](#走って帰る)
+ - [歩いて帰る](#歩いて帰る)

[fmt-kagi-dialogue] 14行目
- あゆみ「じゃあ、また明日ね。」
+ あゆみ: じゃあ、また明日ね。

[fmt-legacy] 20行目
- [SAY speaker=あゆみ]
- 少しだけ寄り道していかない？
+ あゆみ: 少しだけ寄り道していかない？

6 件の変更（--write でファイルに書き戻せます）
1 件は確信が持てないため変換しませんでした（上記を参照）
```

## パターンカバレッジ

| SPEC 7.1 のパターン | rule_id | 登場箇所 |
|---|---|---|
| front matter がない | `fmt-missing-frontmatter` | ファイル先頭 |
| かぎ括弧セリフ | `fmt-kagi-dialogue` | `あゆみ「…」` |
| 丸括弧の内心（宣言済み話者のみ） | `fmt-paren-dialogue` | `先生（…）`（`characters.yaml` に宣言済み） |
| リンクのない中黒リスト | `fmt-linkless-choice` | `・走って帰る` / `・歩いて帰る` |
| 旧記法（確定的に変換できる範囲） | `fmt-legacy` | `[SAY speaker=あゆみ]` + 次行の本文 |

`[SHOW_IMAGE name=classroom_evening]` はどこにも変換されず、`before.md` と同じ行のまま
`after.md` にも残っている。front matter は 1 シーンに `background` を 1 つしか持てず、
アセット系コマンドが複数回出てくる場合の統合先が曖昧なため、fmt は確信が持てない
ものとして変換せず `legacy-command` の Diagnostic だけを報告する（SPEC 7 の
「確信が持てない箇所は変換しない」の実例）。
