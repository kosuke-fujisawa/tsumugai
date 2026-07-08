# 0001. シナリオ記法 v2 への移行を検討する

- 日付: 2026-07-08（採用案確定: 2026-07-09）
- 状態: 採用（v2 への全面移行は不採用。案 A を基本に、必要な改善のみ限定的に案 C として取り込む）

## 背景

外部から tsumugai への要求文書（「tsumugai 実装要求」）が提示され、シナリオ記法の例として以下が示された。

```md
# scene: spring_start

春の教室は、少しだけ騒がしい。

葵: おはよう。今日も早いね。

- [図書室へ行く](scene:library)
- [屋上へ行く](scene:rooftop)

# scene: library

図書室は静かだった。

葵: ここ、落ち着くでしょ。

::ending aoi_good
```

これは tsumugai 現行の v1 記法（SPEC.md、実装済み・稼働中）と構造的に非互換である。

| 項目 | 要求文書の例 | 現行 v1 記法 |
|---|---|---|
| シーン宣言 | `# scene: id`（1 ファイルに複数 H1 を許容しているように見える） | front matter の `id:` で宣言、1 ファイル = 1 シーン、H1 は 1 つのみ（2 つ目は `invalid-h1` error） |
| 分岐先 | `[label](scene:library)`（独自 URI スキーム） | `[label](#anchor)` / `[label](file.md#anchor)`（相対パス・アンカーのみ。`scheme:` 形式は `broken-link` として拒否する既存テストあり） |
| エンディング | `::ending aoi_good`（独自ディレクティブ） | `<!-- ending: id -->`（HTML コメント） |
| 変数操作 | 「明示的な記法で variable 操作を表す」（構文は未指定） | 変数に相当する構文は存在しない |

さらに要求文書は StoryBundle の Step 種別に `set_variable` を含めることも求めており、これは新しい変数構文の設計を前提とする。

v1 記法は以下に依存・統合されている。

- `src/scenario/parse.rs` の実装全体（front matter 解析、H1/H2 の役割、1 ファイル = 1 シーンという前提）
- `examples/spring/`（仕様網羅サンプル。`check` 0 件が既存テストの受け入れ条件）
- `tests/fixtures/` 配下の全フィクスチャ（check / trace / routes / compile の各テスト）
- arikoi 側の `scripts/check-scenario.sh`（旧 CLI 境界の統合、memory 記録によれば本番稼働中）と、arikoi 側の実シナリオファイル（v1 記法で書かれている）
- `compile --target web` が前提とする内部 IR（`Scene { lead, sections }`）

過去に一度、tsumugai 全体を TypeScript / Svelte / Vite へ全面移行する epic #99 が発議されたが、検討の結果「不採用」で決着した経緯がある（PR #133 / #139 / #140、README・CLAUDE.md・AGENTS.md の注記参照）。本件は言語移行ではなくシナリオ記法の移行だが、「破壊的な全面移行」という性質は共通する。

## 選択肢

### A. v1 記法を維持し、要求文書のシナリオ例は概念レベルの例示として扱う

scene / narration / dialogue / choice / jump / ending という **機能レベル** の要求は、現行 v1 記法（front matter + H1 + 相対リンク + HTML コメント）で既に満たされていると解釈する。変数（`set_variable`）だけを、v1 の枠組みに追記可能な形で新設する（例: front matter への `variables:` 宣言 + 新ブロック種別）。

- 長所: 破壊的変更なし。`examples/spring`・全テスト・arikoi 側の既存シナリオがそのまま有効
- 短所: 要求文書の文言（`# scene:` 等）と実装の字面が一致しないため、要求文書だけを読んだ第三者が「未実装」と誤解しうる

### B. 要求文書の記法（v2）へ全面移行する

`# scene: id` による 1 ファイル複数シーン、`scene:xxx` リンク、`::ending id` ディレクティブに合わせてパーサーを書き換える。SPEC.md を v2 として改訂し、既存 examples / tests / fixtures を全面移行する。

- 長所: 要求文書とコードの字面が一致する
- 短所: tsumugai リポジトリ単独では完結しない。arikoi 側のシナリオファイルと `check-scenario.sh` の移行も必要（別リポジトリ）。破壊的変更の範囲が大きく、epic #99 と同様の「大規模移行が途中で不採用になる」リスクを抱える

### C. v1 をベースに、要求文書が重視する書き味だけを段階的に取り込む（v1.1）

1 ファイル複数シーンなど、要求文書が重視する特性を個別に評価し、価値が明確なものだけを既存記法に非破壊的に追加する。

- 長所: 破壊的変更を避けつつ改善できる
- 短所: 「概念レベルの要求」と「文字どおりの記法」の境界が曖昧になりやすく、都度判断が必要

## 採用案

**案 A を基本とし、必要な改善だけを案 C として限定的に取り込む。案 B（v2 全面移行）は不採用。**

- 現行 v1 記法（front matter + H1 + 相対リンク + HTML コメント）を維持する
- `# scene: id` による 1 ファイル複数シーン、`scene:xxx` リンク、`::ending id` ディレクティブへの全面移行は行わない
- 「tsumugai 実装要求」に出てきた記法例は、字面どおりの必須仕様ではなく、scene / narration / dialogue / choice / jump / ending という**機能レベルの要求の例示**として扱う
- arikoi との連携は、記法そのものの一致ではなく、引き続き `compile --target web` による StoryBundle JSON 出力で行う。arikoi 側の runtime / save-load / debug 要件は StoryBundle 側の表現を強化することで満たす

個別の論点は次のとおり決定する。

- **1 ファイル = 1 シーン**: 維持する。`# scene: id` による 1 ファイル複数シーンは採用しない
- **分岐先記法**: 現行の Markdown リンク（`[label](file.md#anchor)` 等）を維持する。`scene:library` のような独自 URI スキームは採用しない
- **エンディング記法**: 現行の `<!-- ending: id -->` を維持する。`::ending id` は採用しない
- **変数構文**: 本 ADR では確定しない。arikoi の実シナリオで変数・条件分岐が実際に必要になった時点で、別 issue として v1 互換の追加記法を設計する（案 C の一部として将来検討）
- **step id**: 作者が明示的に step id を書く記法は現時点で導入しない。ただし StoryBundle 側では、セーブロードとデバッグのために安定した step 識別子を持たせる方向を検討する。これは記法変更ではなく `compile` 出力側（StoryBundle スキーマ）の改善として扱う

## 採用理由

v1 はすでに以下を満たしている。

- Markdown ライクに書ける
- `check` / `trace` / `routes` / `compile` と統合済み
- 1 ファイル = 1 シーンの設計が parser / tests / examples / StoryBundle 出力に組み込まれている
- `compile --target web` により arikoi 側 Svelte player との CLI + JSON 境界を作れている

一方で案 B（v2 全面移行）は、parser・SPEC・fixtures・examples・tests・arikoi 側シナリオまで巻き込む破壊的変更になる。現時点の目的は arikoi の技術スパイクと開発の前進であり、記法移行自体を主目的にすべきではない。理想的な記法よりも、arikoi が動く StoryBundle を安定して出せることを優先する。

## 欠点・リスク

- 採用した案 A: 要求文書と実装の字面が一致しないままだと、レビューする人（Rust を読めない前提の利用者を含む）が誤解する可能性がある。SPEC.md に「外部要求文書の記法例は機能要求の例示であり、v1 では字面どおり採用しない」旨を明記して対応する
- 不採用とした案 B: arikoi 側の移行が前提になるため、tsumugai 側だけで進めると arikoi と齟齬が生じたはずだった。epic #99 は「並行セッションによる issue 重複」という運用上の事故も伴っており、今後同種の大規模移行を検討する際は着手前に最新の issue / PR 状態を確認する運用上の注意が要る

## 再評価条件

- arikoi 側で `# scene:` 記法や `scene:` リンクを具体的に書きたいという要望が出た場合
- 変数（フラグ・条件分岐）が現行 v1 記法で表現できない具体的なシナリオが実際に出てきた場合
- 1 ファイル複数シーンが必要になる具体的な制作上の理由が出てきた場合

## 影響範囲

案 A + 限定的 C を採用したため、`src/scenario/` のパーサー・examples・tests・arikoi 側シナリオへの破壊的変更は発生しない。今回の決定による実際の影響範囲は次のとおり。

- `SPEC.md`: 外部要求文書の記法例（`# scene:` 等）は v1 では字面どおり採用しない旨を追記
- 本 ADR（`docs/adr/0001-scenario-syntax-v2-migration.md`）: 状態を「採用」に更新
- 変数構文・StoryBundle の step 識別子は、必要になった時点でそれぞれ別 issue として切り出す（本 ADR の対象外）
