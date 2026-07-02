//! characters.yaml の探索と読み込み（SPEC 2.1）
//!
//! キャラクター（話者）の事前宣言ファイル。シナリオファイルと同じ
//! ディレクトリ、またはその祖先ディレクトリに置かれ、最も近いものが使われる。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 読み込んだキャラクター定義
#[derive(Debug, Clone, PartialEq)]
pub struct Characters {
    /// 定義ファイルのパス
    pub path: PathBuf,
    /// 話者名 → メタデータ（tsumugai は中身を解釈せず compile 先へ引き渡す）
    pub entries: BTreeMap<String, serde_yaml::Value>,
}

impl Characters {
    pub fn contains(&self, speaker: &str) -> bool {
        self.entries.contains_key(speaker)
    }
}

/// シナリオファイルの位置から characters.yaml を探す。
/// 同階層 → 祖先ディレクトリの順で最も近いものを返す。
pub fn find_characters_file(scene_path: &Path) -> Option<PathBuf> {
    let start = scene_path.parent()?;
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join("characters.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}

/// characters.yaml を読み込む。
///
/// 形式エラーは呼び出し側（check）が Diagnostic に変換できるよう
/// メッセージ文字列で返す。
pub fn load_characters(path: &Path) -> Result<Characters, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("{} を読み込めません: {}", path.display(), e))?;
    parse_characters(&source, path)
}

fn parse_characters(source: &str, path: &Path) -> Result<Characters, String> {
    let value: serde_yaml::Value = serde_yaml::from_str(source)
        .map_err(|e| format!("{} の YAML が解析できません: {}", path.display(), e))?;
    let mapping = value
        .get("characters")
        .and_then(|v| v.as_mapping())
        .ok_or_else(|| {
            format!(
                "{} に `characters:` マッピングがありません。`characters:` の下に話者名を並べてください",
                path.display()
            )
        })?;
    let mut entries = BTreeMap::new();
    for (key, val) in mapping {
        let name = key
            .as_str()
            .ok_or_else(|| format!("{} の話者名が文字列ではありません", path.display()))?;
        entries.insert(name.to_string(), val.clone());
    }
    Ok(Characters {
        path: path.to_path_buf(),
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn characters_yamlを解析できる() {
        let src = "characters:\n  幼なじみ:\n    color: \"#ff9999\"\n  主人公: {}\n";
        let chars = parse_characters(src, Path::new("characters.yaml")).unwrap();
        assert!(chars.contains("幼なじみ"));
        assert!(chars.contains("主人公"));
        assert!(!chars.contains("先生"));
    }

    #[test]
    fn charactersキーがないとエラーになる() {
        let err = parse_characters("cast:\n  A: {}\n", Path::new("c.yaml")).unwrap_err();
        assert!(err.contains("characters"));
    }
}
