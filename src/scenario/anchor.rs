//! H2 見出しからのアンカー名導出（SPEC 3.2）
//!
//! GitHub の slug 生成と同じ規則:
//! 1. 前後の空白を除去し、小文字化する
//! 2. 空白を `-` に置き換える
//! 3. Unicode の文字（Letter）・結合記号（Mark）・数字（Number）・
//!    ハイフン・アンダースコア以外を除去する

/// 見出しテキストをアンカー名（slug）に変換する。
///
/// 空文字列が返る場合、呼び出し側は `empty-anchor` を報告する。
pub fn slugify(heading_text: &str) -> String {
    let mut slug = String::new();
    for ch in heading_text.trim().to_lowercase().chars() {
        if ch.is_whitespace() {
            slug.push('-');
        } else if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            // char::is_alphanumeric は Unicode の Letter / Number を含む
            // （ひらがな・カタカナ・漢字・アクセント付きラテン文字も保持される）
            slug.push(ch);
        }
        // それ以外（記号・句読点など）は除去
    }
    slug
}

/// リンクフラグメントの %-エンコードをデコードする（SPEC 4.3）。
///
/// 日本語見出しへのリンクをエディタが `#%E9%81%B8%E6%8A%9E%E8%82%A2` の
/// ように書き出すことがあるため、解決前にデコードする。
pub fn percent_decode(fragment: &str) -> String {
    let bytes = fragment.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii見出しは小文字ハイフン区切りになる() {
        assert_eq!(slugify("Run Together"), "run-together");
    }

    #[test]
    fn 日本語見出しはそのまま保持される() {
        assert_eq!(slugify("選択肢"), "選択肢");
        assert_eq!(slugify("エピローグ"), "エピローグ");
    }

    #[test]
    fn アクセント付きラテン文字は保持される() {
        assert_eq!(slugify("Café"), "café");
    }

    #[test]
    fn 記号のみの見出しは空になる() {
        assert_eq!(slugify("!?"), "");
    }

    #[test]
    fn パーセントエンコードをデコードできる() {
        assert_eq!(percent_decode("%E9%81%B8%E6%8A%9E%E8%82%A2"), "選択肢");
        assert_eq!(percent_decode("run-together"), "run-together");
    }
}
