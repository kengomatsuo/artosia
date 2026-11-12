//! The made-up song that shows the several-languages screens in the practice library and in the
//! headless scenes. Its words are written for Artosia and are nobody's song.

use crate::song::{Song, Translation};

const ID: &str = "[1]\nPagi datang, kami di sini\nTerima kasih Kau jaga malam kami\nSuara kami kecil, milik Engkau\nTerimalah hari ini seluruhnya\n[chorus]\nKami bersyukur, kami bersyukur\nKebaikan Engkau baru pagi ini\nTerang penuhi ruang, penuhi hati\nKami bersyukur, kami milik Engkau\n[2]\nUntuk roti, untuk sahabat di sisi\nUntuk kerja tangan dan istirahat\nUntuk yang tidak hadir hari ini\nJagalah mereka dan kami, Tuhan\n";

const EN: &str = "[1]\nThe morning is here, we are here.\nYou kept watch over us all night.\nOur voices are small, but Yours.\nTake this day and all within it.\n[chorus]\nWe thank You, we thank You.\nYour kindness is new today.\nLight fills the room, fill us too.\nWe thank You, we are Yours.\n[2]\nFor bread and friends beside us.\nFor work by day, rest at night.\nFor those who are away today.\nKeep them, and us, in Your care.\n";

const ZH: &str = "[1]\n清晨来到，我们聚在这里\n感谢你看顾了整个夜晚\n我们的声音微小，却属于你\n求你收下今天，和今天的一切\n[chorus]\n我们感谢你，我们感谢你\n你的慈爱每天早晨都是新的\n愿光照满这里，也照满我们的心\n我们感谢你，我们属于你\n[2]\n感谢餐桌上的饼，感谢身边的朋友\n感谢手中的工作，和工作后的安息\n今天不在这里的人\n求你扶持他们，也扶持我们\n";

/// One song in Indonesian, English and Simplified Chinese, Indonesian first.
pub fn three_languages() -> Song {
    Song {
        number: 7,
        title: "Syukur di pagi ini".into(),
        alt_title: "Morning of Thanks / 清晨的感谢".into(),
        author: "Artosia practice song".into(),
        order: "V1 C V2 C".into(),
        lyrics: ID.into(),
        lang: "id".into(),
        translations: vec![Translation { lang: "en".into(), text: EN.into() }, Translation { lang: "zh-Hans".into(), text: ZH.into() }],
        ..Song::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_languages_have_the_same_sections() {
        let song = three_languages();
        let tags = |text: &str| crate::song::parse_lyrics(text).iter().map(|s| (s.tag, s.lines.len())).collect::<Vec<_>>();
        let first = tags(&song.lyrics);
        assert_eq!(first.len(), 3);
        assert!(song.translations.iter().all(|t| tags(&t.text) == first));
    }
}
