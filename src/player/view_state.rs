//! ViewState：CUI プレイヤーの表示状態管理
//!
//! 現在の画面状態（背景・BGM など）を追跡し、
//! 前の状態との差分（RenderDelta）を計算して差分表示を実現する。
//!
//! # Undo との関係
//! ViewState のスナップショットを履歴に保持することで、
//! Undo 時に完全復元できる（背景・BGM が消えない）。

use crate::runtime::ir::Event;
use std::collections::HashMap;

/// 現在の画面状態
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// 現在のシーン名
    pub scene: Option<String>,
    /// 表示中の画像（レイヤー名 → 画像名）
    pub images: HashMap<String, String>,
    /// 再生中の BGM
    pub bgm: Option<String>,
}

impl ViewState {
    pub fn new() -> Self {
        Self {
            scene: None,
            images: HashMap::new(),
            bgm: None,
        }
    }

    /// イベント列を適用して差分を返す
    ///
    /// SE はシーン状態に属さないため毎回差分に含める。
    pub fn apply(&mut self, events: &[Event]) -> RenderDelta {
        let mut delta = RenderDelta::new();

        for event in events {
            match event {
                Event::SceneStart { name } => {
                    if self.scene.as_deref() != Some(name.as_str()) {
                        delta.scene_changed = Some(name.clone());
                        self.scene = Some(name.clone());
                    }
                }

                Event::ShowImage { layer, name } => {
                    let changed = self.images.get(layer) != Some(name);
                    if changed {
                        delta
                            .effects
                            .push(format!("画像表示: {} (レイヤー: {})", name, layer));
                        self.images.insert(layer.clone(), name.clone());
                    }
                }

                Event::ClearLayer { layer } => {
                    if self.images.remove(layer).is_some() {
                        delta.effects.push(format!("レイヤークリア: {}", layer));
                    }
                }

                Event::PlayBgm { name } => {
                    if self.bgm.as_deref() != Some(name.as_str()) {
                        delta.effects.push(format!("BGM: {}", name));
                        self.bgm = Some(name.clone());
                    }
                }

                // SE は永続状態を持たない（毎回出力）
                Event::PlaySe { name } => {
                    delta.effects.push(format!("SE: {}", name));
                }

                Event::Wait { duration } => {
                    delta.effects.push(format!("待機: {}秒", duration));
                }

                Event::PlayMovie { name } => {
                    delta.effects.push(format!("ムービー: {}", name));
                }

                Event::Custom { tag, params } => {
                    delta
                        .effects
                        .push(format!("カスタム: {} {:?}", tag, params));
                }

                // Say は ViewState には影響しない（player 側で表示）
                Event::Say { .. } => {}
            }
        }

        delta
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self::new()
    }
}

/// 前の状態からの差分（表示すべき変化）
#[derive(Debug, Clone, PartialEq)]
pub struct RenderDelta {
    /// シーンが変わった場合の新シーン名
    pub scene_changed: Option<String>,
    /// 追加・変更されたエフェクトの説明文
    pub effects: Vec<String>,
}

impl RenderDelta {
    pub fn new() -> Self {
        Self {
            scene_changed: None,
            effects: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.scene_changed.is_none() && self.effects.is_empty()
    }
}

impl Default for RenderDelta {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 同じ画像を2回適用しても差分なし() {
        let mut vs = ViewState::new();
        let events = vec![Event::ShowImage {
            layer: "bg".to_string(),
            name: "forest.png".to_string(),
        }];
        let delta1 = vs.apply(&events);
        assert!(!delta1.is_empty());

        let delta2 = vs.apply(&events);
        assert!(delta2.is_empty()); // 同じ画像なので差分なし
    }

    #[test]
    fn 同じbgmを2回適用しても差分なし() {
        let mut vs = ViewState::new();
        let events = vec![Event::PlayBgm {
            name: "battle.mp3".to_string(),
        }];
        let _ = vs.apply(&events);
        let delta2 = vs.apply(&events);
        assert!(delta2.is_empty());
    }

    #[test]
    fn seは毎回差分に含まれる() {
        let mut vs = ViewState::new();
        let events = vec![Event::PlaySe {
            name: "click.wav".to_string(),
        }];
        let delta1 = vs.apply(&events);
        let delta2 = vs.apply(&events);
        assert!(!delta1.is_empty());
        assert!(!delta2.is_empty()); // SE は毎回
    }

    #[test]
    fn スナップショットによるundo復元() {
        let mut vs = ViewState::new();
        vs.apply(&[Event::ShowImage {
            layer: "bg".to_string(),
            name: "forest.png".to_string(),
        }]);
        vs.apply(&[Event::PlayBgm {
            name: "bgm1.mp3".to_string(),
        }]);

        // スナップショット保存
        let snapshot = vs.clone();

        // さらに変更
        vs.apply(&[Event::ShowImage {
            layer: "bg".to_string(),
            name: "castle.png".to_string(),
        }]);
        assert_eq!(vs.images.get("bg").unwrap(), "castle.png");

        // スナップショット復元
        vs = snapshot;
        assert_eq!(vs.images.get("bg").unwrap(), "forest.png"); // 元に戻る
        assert_eq!(vs.bgm.as_deref(), Some("bgm1.mp3")); // BGM も保持
    }
}
