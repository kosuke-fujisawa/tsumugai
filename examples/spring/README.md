# サンプルシナリオ: spring

SPEC.md（記法 v1）の全要素を網羅する最小プロジェクト。仕様の読み合わせと、#75 以降のパーサー・check のテスト入力に使う。

```text
spring/
├── characters.yaml          # キャラクター定義（SPEC 2.1）
├── scenario/
│   ├── spring_001.md
│   └── spring_002.md
└── assets/                  # front matter から参照される実在チェック用のプレースホルダー（中身は空）
```

## 仕様カバレッジ

| SPEC の節 | 要素 | 登場箇所 |
|---|---|---|
| 2.1 | characters.yaml（メタデータ付き / 空） | `characters.yaml` |
| 3.1 | front matter（id / background / bgm） | 両ファイル先頭 |
| 3.2 | H1 タイトル・H2 セクションとアンカー | 両ファイル |
| 4.1 | ナレーション | 両ファイル |
| 4.2 | セリフ（`名前:` 形式） | 両ファイル |
| 4.3 | 選択肢: ファイル内アンカー | `spring_001.md` `#run-together` |
| 4.3 | 選択肢: 別ファイル | `spring_001.md` → `spring_002.md` |
| 4.3 | 見出しなしの選択肢リスト | `spring_002.md` 冒頭 |
| 4.4 | ジャンプ: 別ファイル内アンカー | `spring_001.md` → `spring_002.md#after-school` |
| 4.4 | ジャンプ: ファイル内アンカー | `spring_002.md` → `#after-school` |
| 4.5 | エンディング（複数） | `childhood_route` / `sprint_route` / `calm_route` |
| 4.5 | メモ用 HTML コメント | `spring_002.md` 冒頭 |
| 5 | 意図したフォールスルー（リード → 最初のセクション） | `spring_001.md` `## 選択肢` |

このプロジェクトは `tsumugai check` で Diagnostic が 0 件になることを想定している（v1 パーサー実装後）。
