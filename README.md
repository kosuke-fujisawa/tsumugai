# tsumugai

Markdownでビジュアルノベルのシナリオを簡単に書けるスクリプトエンジン。

---

## 概要

**tsumugai** は、Markdownで記述したビジュアルノベルのシナリオをパースし、順次実行可能なコマンド列に変換するRustライブラリです。  
**最小限・決定的・アプリ非依存** を方針とし、あらゆる描画・再生環境に組み込める設計になっています。

---

## 責務

1. **Markdownシナリオのパース**
   - `.md` ファイルを読み込み。
   - コマンドは `[COMMAND key=value, key=value]` 形式。
   - コメントは `<!-- ... -->` を使用（LLMにも読みやすい）。

2. **順次実行可能なコマンド列の生成**
   - スクリプト順に上から解釈。
   - 分岐 `[BRANCH ...]` やフラグ条件付き表示をサポート。

3. **アセット参照情報の返却**
   - 音楽・画像・動画のファイル名をそのまま返す。
   - 存在確認やロードは行わない。

4. **実行順序と状態の制御**
   - アプリに実行結果を1ステップずつ返す。
   - 進行度や分岐選択、フラグ更新を内部で保持。

---

## 非責務（やらないこと）

- 画像・音声・動画の描画や再生  
- アセットの存在確認  
- UI表示やレイアウト  
- LLMプロンプト生成  
- フェード・トランジションなど複雑な演出

---

## 副次的メリット

- **LLMフレンドリー**：プレーンテキスト＆構造化Markdownで、LLMによる生成・編集が容易。  
- **Git/Lintフレンドリー**：テキストベースなので差分・Lint・自動テストがしやすい。  
- **クロスプラットフォーム対応**：Rust単体で動作、Tauri/SvelteやUnityバインディングでも利用可能。

---

## 基本の流れ

1. アプリが `.md` シナリオファイルのパスを渡す  
2. tsumugai がパースしてイベント列を返す  
3. アプリが受け取ったイベントを描画・再生する  
4. 分岐選択やパラメータ変更をtsumugaiに渡す  
5. 終了条件まで繰り返す

---

## 想定利用者

- コードだけでビジュアルノベルを開発したいエンジニア  
- Tauri + Svelte など、エディタ依存のない構成を選ぶ開発者  
- Git管理やLLM支援でシナリオ制作を効率化したいチーム

---

## 導入

`Cargo.toml` に追加：

```toml
tsumugai = { git = "https://github.com/yourname/tsumugai", tag = "v0.1.0" }
```
使用例：

**注意**: `Engine::from_markdown` は現在プレースホルダー実装です。完全な統合は将来のバージョンで提供予定です。

```rust
use tsumugai::{Engine, NextAction};

let source = std::fs::read_to_string("script.md")?;
let mut engine = Engine::from_markdown(&source)?;

loop {
    match engine.step()? {
        step_result => {
            // Handle directives (Say, PlayBgm, ShowImage, etc.)
            for directive in step_result.directives {
                println!("Execute: {:?}", directive);
            }
            
            match step_result.next {
                NextAction::Next => continue,
                NextAction::WaitUser => {
                    // Wait for user input (Enter key)
                    wait_for_input();
                }
                NextAction::WaitBranch => {
                    // Show choices and get user selection
                    let choice_index = get_user_choice();
                    engine.choose(choice_index)?;
                }
                NextAction::Halt => break,
            }
        }
    }
}
```
## アーキテクチャの選択

tsumugai には2つの異なるアーキテクチャが同梱されており、用途に応じて選択できます。

### 簡易アーキテクチャ (Facade)
手軽に利用することを目的としたシンプルなAPIです。Markdownパーサーとランタイムが直接結合されており、少ないコードでゲームを動かすことができます。

- **想定ユースケース**: 小規模なゲーム、プロトタイピング、他のアプリケーションへの簡単な組み込み。
- **API**: `tsumugai::facade::Facade` を通じて操作します。

### コアアーキテクチャ (Layered)
拡張性と保守性を重視した、ドメイン駆動設計に基づく階層型アーキテクチャです。`domain`, `application`, `infrastructure` の3層に責務が分離されています。

- **想定ユースケース**: 大規模なゲーム、複雑な独自ロジックの追加、長期的なメンテナンスが必要なプロジェクト。
- **API**: `tsumugai::application::engine::StoryEngine` を中心に、より詳細な制御が可能です。

---

## ライセンス
MIT License

## アーキテクチャ

### アダプタパターンの採用

tsumugaiは段階的な移行のため、以下のアダプタパターンを採用しています：

- **Legacy Adapter** (`src/legacy_adapter.rs`): 旧IR-based APIと新domain-driven実装の橋渡し
- **Resource Adapters** (`src/story_engine.rs`): インフラ層とアプリケーション層間のリソース解決適応
- **Repository Adapters**: ドメインリポジトリとアプリケーション特性間の変換

### プレースホルダ実装

現在の `StoryEngine::from_markdown()` は**プレースホルダ実装**です：

- 実際のパース結果を使わず、空のシナリオを作成
- 新Engine（application/engine.rs）と旧Engine（domain層）の統合は将来対応
- 現在は動作確認と契約検証が目的

## 開発メモ（自分用）
- CIでは全分岐をテストし、ゴールデンデータを更新
- API変更時は API.md を先に更新
- スキーマ更新はコード変更より先に行う
- examples/ 内のサンプルは cargo run --example ... で動く状態を維持
